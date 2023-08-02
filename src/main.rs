#![feature(generators, generator_trait)]
// https://serde.rs/stream-array.html
// seems to be a bit clunky

// actually designed for streaming, but seems a bit incomplete
// https://github.com/Marcono1234/struson

// type StrCon<T> = std::rc::Rc<T>;
type StrCon<T> = Box<T>;

fn make_readable() -> StrCon<dyn std::io::BufRead> {
  let args = std::env::args().collect::<Vec<_>>();
  // use std::io::Read;
  match &args[..] {
    [_] => StrCon::new(std::io::stdin().lock()),
    [_, arg_fn] => {
      let file = std::fs::File::open(arg_fn).expect("cannot open file {arg_fn}");
      StrCon::new(std::io::BufReader::new(file))
    }
    _ => panic!("too many args")
  }
}

struct JsonEvents {
  reader : json_event_parser::JsonReader<Box<dyn std::io::BufRead>>,
  buf : Vec<u8>,
}

impl JsonEvents {
  fn new() -> Self {
    let istream = make_readable();
    let reader = json_event_parser::JsonReader::from_reader(istream);
    let buf : Vec<u8> = vec![];
    Self{reader, buf}
  }

  fn next(&mut self) -> Option<json_event_parser::JsonEvent> {
    match self.reader.read_event(&mut self.buf).unwrap() {
      json_event_parser::JsonEvent::Eof => None,
      event => Some(event),
    }
  }
}

// fn make_json_event_iterator<'l>(istream : &'l mut impl std::io::BufRead) -> impl std::iter::Iterator + 'l {
// fn make_json_event_iterator<'l>() -> impl std::iter::Iterator + 'l {
//   let istream = make_readable();
//   let mut reader = json_event_parser::JsonReader::from_reader(istream);
//   let mut buf : Vec<u8> = vec![];

//   std::iter::from_fn( move || {
//     match reader.read_event(&mut buf).unwrap() {
//       eof @ json_event_parser::JsonEvent::Eof => None,
//       event => Some(event),
//     }
//   })
// }

fn main() {
  // let istream = make_readable();
  // json_event_parser::{JsonReader, JsonEvent};
  // use std::io::Cursor;
  // use std::io::BufRead;


  // let mut reader = json_event_parser::JsonReader::from_reader(Cursor::new(istream));
  // let mut reader = json_event_parser::JsonReader::from_reader(istream);
  let mut json_events = JsonEvents::new();
  while let Some(ev) = json_events.next() {
    println!("{ev:?}");
  }

  // let mut generator = move || {
  //   let mut buf : Vec<u8> = vec![];
  //   loop {
  //     match reader.read_event(&mut buf).unwrap() {
  //       eof @ json_event_parser::JsonEvent::Eof => return eof,
  //       event => {
  //         yield event
  //       }
  //     };
  //   }
  // };

  // use std::ops::Generator;
  // loop {
  //   match std::pin::Pin::new(&mut generator).resume(()) {
  //     std::ops::GeneratorState::Yielded(_jason_event) => (),
  //     std::ops::GeneratorState::Complete(json_event_parser::JsonEvent::Eof) => (),
  //     other => panic!("unexpected yield {other:?}"),
  //   }
  // }
}
