/*!
The interface to the parser, currently json-event-parser.
*/

use std::cell::RefCell;

use crate::plain;

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
  reader : RefCell<json_event_parser::FromReadJsonReader<JsonCounter>>,
  _buf : Vec<u8>,
}

fn fetch_err_context(err : json_event_parser::ParseError, JsonCounter(counter) : &mut JsonCounter) {
  let pos = counter.reader_bytes();
  // try to generate some surrounding json context for the error message
  let mut buf = [0; 80];
  use std::io::Read;
  let more = match counter.read(&mut buf) {
    Ok(n) => String::from_utf8_lossy(&buf[0..n]).to_string(),
    Err(err) => format!("{err:?}"),
  };

  format!("pos {pos} {err} followed by {more}");
}

impl<'l> JsonEvents {
  pub fn new(istream : Box<dyn std::io::BufRead>) -> Self {
    let counter = JsonCounter(countio::Counter::new(istream));
    let reader = json_event_parser::FromReadJsonReader::new(counter);
    Self{reader: RefCell::new(reader), _buf: vec![]}
  }

  // pub fn next_buf<'a>(&'a self, _buf : &mut Vec<u8>) -> Option<json_event_parser::JsonEvent<'a>> {
  pub fn next_buf<'a>(&'a self, _buf : &mut Vec<u8>) -> Option<plain::JsonEvent<String>> {
    let mut binding = self.reader.borrow_mut();
    let jep_event_result = binding.read_next_event().expect("TODO fixme");
    let event_result = plain::JsonEvent::from(jep_event_result);
    Some(event_result)
    // eventicize!(self, event_result)
    // // convert to a type with a self-contained buffer
    // if let Some(ref json_event) = eventicize!(self, event_result) {
    //   Some(crate::plain::JsonEvent::from(json_event))
    // } else {
    //   None
    // }
  }
}

impl Iterator for JsonEvents {
  type Item = crate::plain::JsonEvent<String>;

  fn next(&mut self) -> Option<<Self as Iterator>::Item> {
    // read the event which contains a reference to the buffer in self._buf
    match self.reader.borrow_mut().read_next_event() {
        Ok(jev) => Some(plain::JsonEvent::from(jev)),
        Err(err) => {
          fetch_err_context(err, self.reader.borrow_mut().reader());
          None
        }
    }
  }
}
