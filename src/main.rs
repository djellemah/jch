// parser and traits
mod parser;
mod jsonpath;
mod sendpath;
mod sender;
mod handler;

// handlers and sender implementations
mod plain;
mod shredder;
mod schema;
mod valuer;
mod channel;
mod fn_snd;

use crate::parser::JsonEvents;
use crate::sender::Event;
use crate::sender::Sender;
use crate::jsonpath::Step;
use crate::jsonpath::JsonPath;


fn shred<S>(dir : &std::path::PathBuf, maybe_readable_args : &[S])
where S : AsRef<str> + std::convert::AsRef<std::path::Path> + std::fmt::Debug
{
  let istream = cln::make_readable(maybe_readable_args);
  let mut jevstream = JsonEvents::new(istream);

  // write events as Dremel-style record shred columns
  let mut writer = crate::shredder::ShredWriter::new(&dir, "mpk");

  // serialisation format for columns
  use crate::shredder::MsgPacker;
  use crate::handler::Handler;
  let visitor = MsgPacker::new();

  match visitor.value(&mut jevstream, JsonPath::new(), 0, &mut writer ) {
    Ok(()) => (),
    Err(err) => { eprintln!("ending event reading because {err:?}") },
  }
}

fn main() {
  // Quick'n'Dirty command line arg dispatch
  let args : Vec<String> = std::env::args().collect();
  let args : Vec<&str> = args.iter().map(String::as_str).collect();
  match &args[1..] {
    ["-s", "-z"] => schema::sizes(),
    ["-s", rst @ ..] => {
      let istream = cln::make_readable(rst);
      let mut jevstream = JsonEvents::new(istream);
      schema::schema(&mut jevstream);
    }
    ["-p", rst @ ..] => {
      let istream = cln::make_readable(rst);
      let mut jevstream = JsonEvents::new(istream);

      // just use a (mostly) simple function wrapper
      // which just outputs the value if sent.
      let sender = &mut fn_snd::FnSnd(|ev| Ok::<(),()>(println!("fn_snd {ev:?}")));
      // always returns true for path matches
      let visitor = plain::Plain;

      use handler::Handler;
      visitor
        .value(&mut jevstream, JsonPath::new(), 0, sender)
        .unwrap_or_else(|err| eprintln!("ending event reading because {err:?}"));
    }
    ["-v", rst @ ..] => {
      let istream = cln::make_readable(rst);
      let mut jevstream = JsonEvents::new(istream);

      // accept all paths, and convert leafs to serde_json::Value
      let visitor = valuer::Valuer(|_path| true);
      // just print them out
      // let sender = &mut valuer::ValueSender;
      let sender = &mut fn_snd::FnSnd(|ev| Ok::<(),()>(println!("{ev:?}")));
      // go and doit
      use handler::Handler;
      visitor
        .value(&mut jevstream, JsonPath::new(), 0, sender)
        .unwrap_or_else(|err| eprintln!("ending event reading because {err:?}"));
    }
    ["-c", rst @ ..] => {
      let istream = cln::make_readable(rst);
      let mut jevstream = JsonEvents::new(istream);
      // producer reads file and converts to serde_json events, consumer just receives them.
      channel::channels(&mut jevstream)
    }
    ["-h"] => println!("-z file for sizes, -s file for schema"),
    _ =>  {
      println!("-s [file] for schema\n-p [file] for plain\n-v [file] for valuer\n-c [file] for channel\nTODO resurrect shredder");
      std::process::exit(1)
    }
  }
}
