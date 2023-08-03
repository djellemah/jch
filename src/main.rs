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
  fn new(istream : StrCon<dyn std::io::BufRead>) -> Self {
    let reader = json_event_parser::JsonReader::from_reader(istream);
    let buf : Vec<u8> = vec![];
    Self{reader, buf}
  }

  // it's a severe PITA to specify this as an implementation of Iterator
  fn next(&mut self) -> Option<json_event_parser::JsonEvent> {
    match self.reader.read_event(&mut self.buf).unwrap() {
      json_event_parser::JsonEvent::Eof => None,
      event => Some(event),
    }
  }
}

// json_event_parser::JsonEvent::String(_) => todo!(),
// json_event_parser::JsonEvent::Number(_) => todo!(),
// json_event_parser::JsonEvent::Boolean(_) => todo!(),
// json_event_parser::JsonEvent::Null => todo!(),
// json_event_parser::JsonEvent::StartArray => todo!(),
// json_event_parser::JsonEvent::EndArray => todo!(),
// json_event_parser::JsonEvent::StartObject => todo!(),
// json_event_parser::JsonEvent::EndObject => todo!(),
// json_event_parser::JsonEvent::ObjectKey(_) => todo!(),
// json_event_parser::JsonEvent::Eof => todo!(),

use json_event_parser::JsonEvent;

// This is basically xpath or jql in disguise
#[allow(dead_code)]
fn ignore(jev : &mut JsonEvents) {
  while let Some(ev) = jev.next() {
    match ev {
      JsonEvent::StartObject => ignore(jev),
      JsonEvent::EndObject => return,
      _ => (),
    }
  }
}

#[allow(dead_code)]
fn handle_top_level(jev : &mut JsonEvents) {
  while let Some(ev) = jev.next() {
    match ev {
      JsonEvent::StartObject => ignore(jev),
      JsonEvent::EndObject => return,
      JsonEvent::ObjectKey(key) => eprintln!("{key}"),
      JsonEvent::Eof => panic!("unexpected eof"),
      _ => (),
    }
  }
}

#[allow(dead_code)]
fn show_all(jev : &mut JsonEvents) {
  while let Some(ev) = jev.next() {
    println!("{ev:?}");
  }
}

fn is_object(jev : &mut JsonEvents) -> bool {
  while let Some(ev) = jev.next() {
    match ev {
      JsonEvent::StartObject => return true,
      _ => return false,
    }
  }
  false
}

fn next_key(jev : &mut JsonEvents, key : &str) {
  let mut keys = std::collections::BTreeSet::<String>::new();
  if is_object(jev) {
    keys.insert(key.to_string());
    find_paths(jev)
  }
}

#[allow(dead_code)]
fn find_paths(jev : &mut JsonEvents) {
  // let jev = std::rc::Rc::new(jev);
  let _keys = std::collections::BTreeSet::<String>::new();

  while let Some(ev) = jev.next() {
    match ev {
      JsonEvent::ObjectKey(key) => next_key(jev, key),
      _ => (),
    }
  }
}

fn main() {
  let istream = make_readable();
  let mut jev = JsonEvents::new(istream);
  while let Some(ev) = jev.next() {
    match ev {
      JsonEvent::StartObject => handle_top_level(&mut jev),
      JsonEvent::ObjectKey(key) => eprintln!("{key}"),
      JsonEvent::Eof => break,
      _ => (),
    }
  }
}
