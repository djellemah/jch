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

Low-level https://github.com/oxidize-rb/rb-sys
High-level https://github.com/matsadler/magnus

Lua
https://github.com/mlua-rs/mlua

That rust prolog
*/

mod ruby {
  use jch::jsonpath;
  use jch::jsonpath::JsonPath;
  use jch::parser::JsonEvent;
  use jch::sender::Sender;

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

  type SendValue = jch::parser::JsonEvent<String>;
  type SendEvent = jch::sender::Event<SendValue>;
  type SendWrapper = jch::sender::NonWrap<SendEvent>;

  impl<'l> jch::handler::Handler<'l, SendValue, SendWrapper, dyn jch::sender::Sender<SendEvent,SendWrapper> + 'l>
  for RubyHandler<'l>
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
    fn maybe_send_value(&self, path : &JsonPath, ev : JsonEvent<std::string::String>, tx : &mut (dyn Sender<SendEvent, SendWrapper> + 'l))
    -> Result<(),Box<dyn std::error::Error>>
    {
      if self.match_path(path) {
        use jch::sender::Event;
        tx
          .send(Event::Value(path.into(), ev.clone()).into())
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

  - 13   seconds with eval and path_ary.length >= 7
  -  5   seconds with ruby_self path_ary.length >= 7
  -  4.1 seconds with yjit/rjit
  */

  // Oh ffs. This is buried inside magnus, which does not expose the relevant
  // functionality. So just fall back to rb_sys and ignore the errors for now.
  //
  // https://ruby-doc.org/3.3.0/yjit/yjit_md.html
  //
  // NOTE can also use the RUBY_YJIT_ENABLE env var - spefically for cases where
  // command-line options are not accessible.
  //
  // You can also enable YJIT at run-time using `RubyVM::YJIT.enable`. This can
  // allow you to enable YJIT after your application is done booting, which
  // makes it possible to avoid compiling any initialization code.
  //
  // You can verify that YJIT is enabled using RubyVM::YJIT.enabled?
  unsafe fn init_ruby(opts : &[&str]) -> magnus::Ruby {
    // this is from magnus::setup
    let mut variable_in_this_stack_frame: rb_sys::VALUE = 0;
    rb_sys::ruby_init_stack(&mut variable_in_this_stack_frame as *mut rb_sys::VALUE as *mut _);
    if rb_sys::ruby_setup() != 0 {
        panic!("Failed to setup Ruby");
    };

    // ok now do the part from init/init_options
    // except skip the error handling cos it's also buried. So if this fails we fall back to debug-by-guesswork :-|
    use std::ffi::CString;
    let mut argv = vec![CString::new("ruby").unwrap()];
    argv.extend(opts.iter().map(|s| CString::new(*s).unwrap()));
    let mut argv = argv
        .iter()
        .map(|cs| cs.as_ptr() as *mut _)
        .collect::<Vec<_>>();

    let node = rb_sys::bindings::uncategorized::ruby_process_options(argv.len() as i32, argv.as_mut_ptr());

    magnus::Ruby::get_unchecked().qnil();

    if rb_sys::ruby_exec_node(node) != 0 {
        panic!("Ruby init code failed");
    };
    magnus::Ruby::get_unchecked()
  }

  pub fn main() {
    // using jit is definitely faster. about 20%
    // rjit is still apparently a bit experimental.
    // --yjit also works here, but is a tad slower.
    // let ruby = unsafe {init_ruby(&["-e", "", "--rjit", "--rjit-stats"])};
    // let ruby = unsafe {init_ruby(&["-e", "", "--yjit", "--yjit-stats"])};
    // let ruby = unsafe {init_ruby(&["-e", "", "--yjit"])};
    let ruby = unsafe {init_ruby(&["-e", "", "--rjit"])};
    let args : Vec<String> = std::env::args().collect();
    filter(&ruby, &args[1]);
    unsafe { rb_sys::ruby_cleanup(0); }
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
