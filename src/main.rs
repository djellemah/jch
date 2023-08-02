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

  fn next(&mut self) -> Option<json_event_parser::JsonEvent> {
    match self.reader.read_event(&mut self.buf).unwrap() {
      json_event_parser::JsonEvent::Eof => None,
      event => Some(event),
    }
  }
}

fn main() {
  let istream = make_readable();
  let mut json_events = JsonEvents::new(istream);
  while let Some(ev) = json_events.next() {
    println!("{ev:?}");
  }
}
