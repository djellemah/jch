/*!
Callback to scripting language for path matching
*/

/*
Elixir
https://github.com/Qqwy/elixir-rustler_elixir_fun
https://elixirforum.com/t/rustlerelixirfun-calling-elixir-from-rust/47984

PyO3


Ruby


Lua
https://github.com/mlua-rs/mlua

That rust prolog
*/

mod ruby {
  use jch::parser;
  use jch::plain;
  use jch::fn_snd;
  use jch::handler;
  use jch::jsonpath;

  #[allow(dead_code)]
  fn canary(ruby : & magnus::Ruby) {
    ruby.eval::<magnus::value::Value>(r#"canary 'hello'"#).expect("ruby can't exec function canary hello");
    let msg = "this is a rust string";
    let _ : magnus::value::Value = magnus::eval![r#"canary msg"#, msg = msg].expect("ruby can't exec function canary with arg");
  }

  pub fn filter(ruby : & magnus::Ruby, json_file_name : &str) {
    let istream = jch::make_readable(&[json_file_name]);
    let mut jevstream = parser::JsonEvents::new(istream);

    let sender = &mut fn_snd::FnSnd( |ev| { println!("ruby_fn_snd {ev:?}"); Ok::<(),String>(())} );

    // TODO will need to set LOAD_PATH for this to work
    // ruby.require("ch").expect("can't require ruby ch(.rb)");
    ruby.eval::<bool>(r#"load "src/bin/ch.rb""#).expect("ruby can't load ch.rb");

    // Sends things as copies rather than references, and always returns true for path matches.
    let visitor = plain::Plain(|path| {
      use magnus::RArray;
      let path_iter = path
        .iter()
        // TODO serde this into ruby hash with {index:, key: value:}
        // or something like that.
        .map(|step| step.to_string());
      let path = RArray::from_iter(path_iter);
      // TODO a more direct way to call the method, ie without eval
      let filter_it : bool = magnus::eval![r#"filter_path a"#, a = path].expect("ruby can't exec function filter_path");
      filter_it
    });

    use handler::Handler;
    visitor
      .value(&mut jevstream, jsonpath::JsonPath::new(), 0, sender)
      // TODO send this error back to ruby?
      .unwrap_or_else(|err| eprintln!("ending event reading because {err:?}"));
  }

  pub fn main() {
    magnus::Ruby::init(|ruby| {
      let args : Vec<String> = std::env::args().collect();
      filter(&ruby, &args[1]);
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
