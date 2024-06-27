/*!
Callback to scripting language for path matching
*/

/*
Elixir
https://github.com/Qqwy/elixir-rustler_elixir_fun
https://elixirforum.com/t/rustlerelixirfun-calling-elixir-from-rust/47984

PyO3


# Ruby

Nah. Cmon people srsly. Direct ffi is not what I'm looking for.
https://dev.to/leehambley/sending-complex-structs-to-ruby-from-rust-4e61

rutie is a bit weird
https://github.com/danielpclark/rutie#using-ruby-in-rust
rutie = "0.8"

Lua
https://github.com/mlua-rs/mlua

That rust prolog
*/

mod ruby {
  // use std::intrinsics::pref_align_of;

use jch::sender::Sender;
  use jch::jsonpath;
  use jch::jsonpath::JsonPath;
  use jch::parser::JsonEvent;

  use jch::fn_snd;
  use jch::handler;

  #[allow(dead_code)]
  fn canary(ruby : & magnus::Ruby) {
    ruby.eval::<magnus::value::Value>(r#"canary 'hello'"#).expect("ruby can't exec function canary hello");
    let msg = "this is a rust string";
    let _ : magnus::value::Value = magnus::eval![r#"canary msg"#, msg = msg].expect("ruby can't exec function canary with arg");
  }

  #[allow(dead_code)]
  struct RubyHandler<'l> (& 'l magnus::Ruby);

  type SendValue = jch::parser::JsonEvent<String>;

  impl<'l> RubyHandler<'l> {
    fn new(ruby : & 'l magnus::Ruby) -> Self {
      // RUBY_YJIT_ENABLE= yes/true/1 no/false/0
      // let jit_enabled : bool = ruby.eval::<bool>("RubyVM::YJIT.enabled?").expect("RubyVM::YJIT.enabled?");
      // println!("jit_enabled: {jit_enabled}");

      // TODO will need to set LOAD_PATH for this to work
      // ruby.require("ch").expect("can't require ruby ch(.rb)");
      ruby.eval::<bool>(r#"load "src/bin/ch.rb""#).expect("ruby can't load ch.rb");
      RubyHandler(ruby)
    }
  }

  impl<'l> jch::handler::Handler<'l, dyn jch::sender::Sender<SendValue> + 'l,SendValue> for RubyHandler<'l>
  where SendValue: std::fmt::Debug + Clone
  {
    fn match_path(&self, path : &JsonPath) -> bool {
      use magnus::value::ReprValue; // for funcall
      let path_iter = path
        .iter()
        // TODO serde this into ruby hash with {index:, key: value:}
        // or something like that.
        .map(|step| step.to_string());

      let ruby_self = magnus::eval::<magnus::Value>("self").expect("ruby can't find self");
      let filter_path_symbol = magnus::value::StaticSymbol::new("filter_path");

      let filter_it : bool = ruby_self
        .funcall(filter_path_symbol, [magnus::RArray::from_iter(path_iter)])
        .expect("failed to call filter path");
      filter_it
    }

    /// send the event provided the fn at self.0 returns true
    fn maybe_send_value(&self, path : &JsonPath, ev : &JsonEvent<String>, tx : &mut (dyn Sender<SendValue> + 'l))
    -> Result<(),Box<dyn std::error::Error>>
    {
      if self.match_path(path) {
        use jch::sender::Event;
        tx
          .send(Box::new(Event::Value(path.into(), ev.clone())))
          .unwrap_or_else(|err| eprintln!("error sending {ev:?} because {err:?}"))
      }
      // ;
      Ok(())
    }
  }

  pub fn filter(ruby : & magnus::Ruby, json_file_name : &str) {
    let istream = jch::make_readable(&[json_file_name]);
    let mut jevstream = jch::parser::JsonEventParser::new(istream);

    let sender = &mut fn_snd::FnSnd( |ev| { println!("ruby_fn_snd {ev:?}"); Ok(Ok::<(),String>(())?)} );

    // Sends things as copies rather than references, and always returns true for path matches.
    let visitor = RubyHandler::new(ruby);

    use handler::Handler;
    visitor
      .value(&mut jevstream, jsonpath::JsonPath::new(), 0, sender)
      // TODO send this error back to ruby?
      .unwrap_or_else(|err| eprintln!("ending event reading because {err:?}"));
  }

  /*
  Benchmarks:

  - 13 seconds with eval and path_ary.length >= 7
  -  5 seconds with ruby_self path_ary.length >= 7
  */

  pub fn main() {
    magnus::Ruby::init(|ruby| {
      let args : Vec<String> = std::env::args().collect();
      filter(ruby, &args[1]);
      Ok(())
    }).unwrap();
  }
}

/* run with
RUST_BACKTRACE=1 \
LD_LIBRARY_PATH=/usr/local/rvm/rubies/ruby-3.3.0/lib \
cargo run --bin ch ../data/whataever.json
*/
fn main() {
  ruby::main()
}
