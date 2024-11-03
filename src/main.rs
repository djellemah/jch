use jch::plain;
use jch::channel;
use jch::shredder;
use jch::parser;
use jch::jsonpath;
use jch::schema;
use jch::handler;
use jch::fn_snd;

use std::marker::PhantomData;
use std::process::exit;

/// The most useful thing this does is calculate a Schema for a json file. Really fast.
/// The rest of it is a showcase and testbed for some of the other things that can be done.
#[allow(clippy::unit_arg)]
fn main() {
  // Quick'n'Dirty command line arg dispatch
  let args : Vec<String> = std::env::args().collect();
  let args : Vec<&str> = args.iter().map(String::as_str).collect();
  match &args[1..] {
    ["-s", "-z"] => schema::sizes(&mut std::io::stdout()).unwrap(),
    ["-s", rst @ ..] => {
      let istream = jch::make_readable(rst);
      let mut jevstream = parser::JsonEventParser::new(istream);
      schema::schema(&mut std::io::stdout(), &mut jevstream);
    }
    // This is PoC to see that the rest of the handlers and visitors work.
    ["-p", rst @ ..] => {
      let istream = jch::make_readable(rst);
      let mut jevstream = parser::JsonEventParser::new(istream);

      // just use a (mostly) simple function wrapper
      // which just outputs the value if sent.
      // kak syntax.
      let sender = &mut fn_snd::FnSnd( |ev| { println!("fn_snd {ev:?}"); Ok(())} );

      // Sends things as copies rather than references, and always returns true for path matches.
      let visitor = plain::Plain(|_| true, PhantomData::<_>);

      use handler::Handler;
      visitor
        .value(&mut jevstream, jsonpath::JsonPath::new(), 0, sender)
        .unwrap_or_else(|err| eprintln!("ending event reading because {err:?}"));
    }
    ["-v", rst @ ..] => {
      use jch::valuer;
      let istream = jch::make_readable(rst);
      let mut jevstream = parser::JsonEventParser::new(istream);

      // accept all paths, and convert leafs to serde_json::Value
      let visitor = valuer::Valuer(|_path| true);

      // just print the output from visitor, which understands serde_json Value
      let sender : &mut fn_snd::FnSnd<serde_json::value::Value> = &mut fn_snd::FnSnd(|ev| Ok(println!("{ev:?}")));

      // go and doit
      use handler::Handler;
      visitor
        .value(&mut jevstream, jsonpath::JsonPath::new(), 0, sender)
        .unwrap_or_else(|err| {eprintln!("ending event reading because {err:?}"); exit(1)})
    }
    ["-c", rst @ ..] => {
      let istream = jch::make_readable(rst);
      let mut jevstream = parser::JsonEventParser::new(istream);
      // producer reads file and converts to serde_json events, consumer just receives them.
      channel::channels(&mut jevstream)
    }
    [ "-m", "-c", dir, rst @ ..] => shredder::channel_shred(&std::path::PathBuf::from(dir), rst),
    [ "-m", dir, rst @ ..] => shredder::shred(&std::path::PathBuf::from(dir), rst),
    [ "-r", "-f", filename, _rst @ ..] => jch::rapid::schema_from_file(filename),
    [ "-r", rst @ ..] => {
      let istream = jch::make_readable(rst);
      jch::rapid::parse(istream)
    }
    _ =>  {
      println!("-s [file] for schema\n-p [file] for plain\n-v [file] for valuer\n-c [file] for channel\n-m <dir> for shredder to MessagePack\n-m -c [dir] for fast shredder to MessagePack\n-r for RapidJson wrapper");
      exit(1)
    }
  }
}
