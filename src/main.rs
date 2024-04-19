use cln::plain;
use cln::valuer;
use cln::channel;
use cln::shredder;
use cln::parser;
use cln::jsonpath;
use cln::schema;
use cln::handler;
use cln::fn_snd;

use std::process::exit;

fn main() {
  // Quick'n'Dirty command line arg dispatch
  let args : Vec<String> = std::env::args().collect();
  let args : Vec<&str> = args.iter().map(String::as_str).collect();
  match &args[1..] {
    ["-s", "-z"] => schema::sizes(&mut std::io::stdout()).unwrap(),
    ["-s", rst @ ..] => {
      let istream = cln::make_readable(rst);
      let mut jevstream = parser::JsonEvents::new(istream);
      schema::schema(&mut jevstream);
    }
    ["-p", rst @ ..] => {
      let istream = cln::make_readable(rst);
      let mut jevstream = parser::JsonEvents::new(istream);

      // just use a (mostly) simple function wrapper
      // which just outputs the value if sent.
      let sender = &mut fn_snd::FnSnd(|ev| Ok::<(),()>(println!("fn_snd {ev:?}")));
      // always returns true for path matches
      let visitor = plain::Plain;

      use handler::Handler;
      visitor
        .value(&mut jevstream, jsonpath::JsonPath::new(), 0, sender)
        .unwrap_or_else(|err| eprintln!("ending event reading because {err:?}"));
    }
    ["-v", rst @ ..] => {
      let istream = cln::make_readable(rst);
      let mut jevstream = parser::JsonEvents::new(istream);

      // accept all paths, and convert leafs to serde_json::Value
      let visitor = valuer::Valuer(|_path| true);
      // just print them out
      // let sender = &mut valuer::ValueSender;
      let sender = &mut fn_snd::FnSnd(|ev| Ok::<(),()>(println!("{ev:?}")));
      // go and doit
      use handler::Handler;
      visitor
        .value(&mut jevstream, jsonpath::JsonPath::new(), 0, sender)
        .unwrap_or_else(|err| {eprintln!("ending event reading because {err:?}"); exit(1)})
    }
    ["-c", rst @ ..] => {
      let istream = cln::make_readable(rst);
      let mut jevstream = parser::JsonEvents::new(istream);
      // producer reads file and converts to serde_json events, consumer just receives them.
      channel::channels(&mut jevstream)
    }
    ["-h"] => println!("-z file for sizes, -s file for schema"),
    [ dir, rst] => shredder::shred(&std::path::PathBuf::from(dir), &[*rst]),
    [ dir, rst @ ..] => shredder::shred(&std::path::PathBuf::from(dir), rst),
    _ =>  {
      println!("-s [file] for schema\n-p [file] for plain\n-v [file] for valuer\n-c [file] for channel\nTODO resurrect shredder");
      exit(1)
    }
  }
}
