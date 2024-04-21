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
use jch::parser;
use jch::plain;
use jch::fn_snd;
use jch::handler;
use jch::jsonpath;

#[allow(unused_imports)]

fn filter(ruby : & magnus::Ruby, json_file_name : &str) {
  let istream = jch::make_readable(&[json_file_name]);
  let mut jevstream = parser::JsonEvents::new(istream);

  // TODO something useful with this - probably call lua
  let sender = &mut fn_snd::FnSnd( |ev| { println!("lua_fn_snd {ev:?}"); Ok::<(),String>(())} );

  // TODO will need to set LOAD_PATH for this to work
  // ruby.require("ch").expect("can't require ruby ch(.rb)");
  ruby.eval::<bool>(r#"load "ch.rb""#).expect("ruby can't load ch.rb");
  ruby.eval::<magnus::r_hash::RHash>(r#"filter_path 'hello'"#).expect("ruby can't exec function filter_path");
  let msg = "this is a rust string";
  let _rg : magnus::r_hash::RHash = magnus::eval![r#"filter_path msg"#, msg = msg].expect("ruby can't exec function filter_path");

  // Sends things as copies rather than references, and always returns true for path matches.
  // TODO call lua function here
  let visitor = plain::Plain(|path| {
    let path = format!("{path}");
    let _rg : magnus::r_hash::RHash = magnus::eval![r#"filter_path a"#, a = path].expect("ruby can't exec function filter_path");
    // ruby.eval::<magnus::r_hash::RHash>(r#"filter_path 'hello'"#).expect("ruby can't exec function filter_path");
    false
  });

  use handler::Handler;
  visitor
    .value(&mut jevstream, jsonpath::JsonPath::new(), 0, sender)
    // TODO send this error back to lua?
    .unwrap_or_else(|err| eprintln!("ending event reading because {err:?}"));
}

#[allow(dead_code)]
fn example() {
}

fn main() {
  magnus::Ruby::init(|ruby| {
    let args : Vec<String> = std::env::args().collect();
    filter(&ruby, &args[1]);
    Ok(())
  }).unwrap();
}
