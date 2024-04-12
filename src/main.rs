mod jsonpath;
mod handler;
mod shredder;
mod sendpath;
mod parser;

use crate::parser::StrCon;
use crate::parser::JsonEvents;
use crate::sendpath::Event;
use crate::sendpath::Sender;
use crate::jsonpath::Step;
use crate::jsonpath::JsonPath;

fn make_readable<S>(maybe_readable_args : &[S]) -> StrCon<dyn std::io::BufRead>
where S : AsRef<str> + std::convert::AsRef<std::path::Path> + std::fmt::Debug
{
  // use std::io::Read;
  match &maybe_readable_args[..] {
    [] => StrCon::new(std::io::stdin().lock()),
    [arg_fn] => {
      let file = std::fs::File::open(arg_fn).expect("cannot open file {arg_fn}");
      StrCon::new(std::io::BufReader::new(file))
    }
    _ => panic!("too many args {maybe_readable_args:?}")
  }
}


#[allow(dead_code, unused_mut, unused_variables)]
fn show_jq_paths() {
  let args = std::env::args().collect::<Vec<String>>();
  let istream = make_readable(&args[..]);
  let mut jev = JsonEvents::new(istream);
  // ch_snd::channels(&mut jev);

  // return true if handler should be called for this path
  let match_lp_ps_path = |json_path : &JsonPath| {
    if json_path.len() < 3 {return false};

    let trefix = json_path
      .iter()
      .take(3)
      .collect::<Vec<&Step>>();

    // This is pretty horrible. Maybe a DSL would be nicer.
    match &trefix[0..3] {
      [&Step::Key(ref v), &Step::Index(n), &Step::Key(ref u) ] => {
        (n == 1 || n == 3) &&
        &v[..] == "learning_paths" &&
        &u[..] == "problem_sequence"
      }
      _ => false
    }
  };
  // fn_snd::values(&mut jev, match_lp_ps_path);

  // let match_images_path = |json_path : &JsonPath| {
  //   // This is pretty horrible. Maybe a DSL would be nicer.
  //   match &json_path[..] {
  //     [&Step::Key(ref v)] => &v[..] == "images",
  //     _ => false
  //   }
  // };
}

fn shred<S>(dir : &std::path::PathBuf, maybe_readable_args : &[S])
where S : AsRef<str> + std::convert::AsRef<std::path::Path> + std::fmt::Debug
{
  let istream = make_readable(maybe_readable_args);
  let mut jev = JsonEvents::new(istream);

  // write events as Dremel-style record shred columns
  let mut writer = crate::shredder::ShredWriter::new(&dir, "mpk");

  // serialisation format for columns
  use crate::shredder::MsgPacker;
  use crate::handler::Handler;
  let visitor = MsgPacker::new();

  match visitor.value(&mut jev, JsonPath::new(), 0, &mut writer ) {
    Ok(()) => (),
    Err(err) => { eprintln!("ending event reading because {err:?}") },
  }
}

mod schema;

fn main() {
  let args : Vec<String> = std::env::args().collect();
  let args : Vec<&str> = args.iter().map(String::as_str).collect();
  match &args[1..] {
    ["-s", rst @ ..] => {
      let istream = make_readable(rst);
      let mut jev = JsonEvents::new(istream);
      schema::schema(&mut jev);
    }
    ["-z"] => schema::sizes(),
    ["-h"] => println!("-z file for sizes, -s file for schema"),
    [] => panic!("you must provide data dir for files"),
    [ dir, rst @ ..] => shred(&std::path::PathBuf::from(dir), rst),
    // _ => panic!("only one data dir needed"),
  }
}
