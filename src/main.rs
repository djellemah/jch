#![feature(generators, generator_trait)]
// https://serde.rs/stream-array.html
// seems to be a bit clunky

// actually designed for streaming, but seems a bit incomplete
// https://github.com/Marcono1234/struson

//
// https://github.com/alexmaco/json_stream

fn make_readable() -> Box<dyn std::io::BufRead> {
  let args = std::env::args().collect::<Vec<_>>();
  // use std::io::Read;
  match &args[..] {
    [_] => Box::new(std::io::stdin().lock()),
    [_, arg_fn] => {
      let file = std::fs::File::open(arg_fn).expect("cannot open file {arg_fn}");
      Box::new(std::io::BufReader::new(file))
    }
    _ => panic!("too many args")
  }
}

fn main() {
  let istream = make_readable();
  // json_event_parser::{JsonReader, JsonEvent};
  // use std::io::Cursor;
  // use std::io::BufRead;


  // let mut reader = json_event_parser::JsonReader::from_reader(Cursor::new(istream));
  let mut reader = json_event_parser::JsonReader::from_reader(istream);
  let mut buf : Vec<u8> = vec![];
  loop {
    match reader.read_event(&mut buf).unwrap() {
      json_event_parser::JsonEvent::Eof => break,
      event => println!("{event:?}"),
    }
  }
}
