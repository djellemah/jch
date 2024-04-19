pub struct JsonCounter(countio::Counter<Box<dyn std::io::BufRead>>);

impl std::io::Read for JsonCounter {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    self.0.read(buf)
  }
}

impl std::io::BufRead for JsonCounter {
  fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
    self.0.fill_buf()
  }

  fn consume(&mut self, amt: usize) {
    self.0.consume(amt)
  }
}

// Source of Json parse events, ie the json parser
pub struct JsonEvents {
  reader : json_event_parser::JsonReader<JsonCounter>,
  _buf : Vec<u8>,
}

// Sheesh. Trying to this with lifetimes is *severe* PITA
macro_rules! eventicize {
  ($obj:expr, $event_result:expr) => {
    match $event_result {
      Ok(json_event_parser::JsonEvent::Eof) => None,
      Ok(jev) => Some(jev),
      Err(err) => {
        // this requires a hack in json_event_parser
        let counter : &JsonCounter = &$obj.reader.reader;
        let pos = counter.0.reader_bytes();
        // try to generate some surrounding json context for the error message
        let mut buf = [0; 80];
        use std::io::Read;
        let more = match $obj.reader.reader.read(&mut buf) {
          Ok(n) => String::from_utf8_lossy(&buf[0..n]).to_string(),
          Err(err) => format!("{err:?}"),
        };

        eprintln!("pos {pos} {err} followed by {more}");

        // Some(json_event_parser::JsonEvent::Null)
        None
      }
    }
  }
}

impl JsonEvents {
  pub fn new(istream : Box<dyn std::io::BufRead>) -> Self {
    let counter = JsonCounter(countio::Counter::new(istream));
    let reader = json_event_parser::JsonReader::from_reader(counter);
    Self{reader, _buf: vec![]}
  }

  // This is an attempt to use JsonEvents as an iterator.
  // But it's a severe PITA to specify this as an implementation of Iterator,
  // and it's impossible to pass errors back.
  #[allow(dead_code)]
  fn next(&mut self) -> Option<json_event_parser::JsonEvent> {
    let event_result = self.reader.read_event(&mut self._buf);
    eventicize!(self, event_result)
  }

  pub fn next_buf<'a, 'b>(&'a mut self, buf : &'b mut Vec<u8>) -> Option<json_event_parser::JsonEvent<'b>> {
    let event_result = self.reader.read_event(buf);
    eventicize!(self, event_result)
  }
}
