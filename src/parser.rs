/*!
The interface to the parser, currently json-event-parser.
*/

use std::cell::RefCell;

/// Mirror of `json_event_parser::JsonEvent`
/// But which doesn't have the `&str` reference into a buffer.
/// Consequently it must be entirely cloned.
/// Which is exactly what implements allows it to implement the `Send` trait.
#[derive(Eq, PartialEq, Debug, Clone, Hash, Copy)]
pub enum JsonEvent<T>
where T : AsRef<[u8]> // because we want storage
{
    String(T),
    Number(T),
    Boolean(bool),
    Null,
    StartArray,
    EndArray,
    StartObject,
    EndObject,
    ObjectKey(T),
    Eof,
}

#[test]
fn create() {
  let _je = JsonEvent::String("hello".to_string());
  let _je = JsonEvent::Number("5".to_string());
  let _je = JsonEvent::ObjectKey("cle".to_string());
}

impl<'a, T> From<&json_event_parser::JsonEvent<'_>> for JsonEvent<T>
where T : AsRef<[u8]> + From<String> // because we want storage + conversion from Cow<'_,str>
{
  fn from<'b>(jev: &'b json_event_parser::JsonEvent<'_>) -> Self {
    use json_event_parser as jep;
    match jev {
      jep::JsonEvent::String(v)    => JsonEvent::String(T::from(v.to_string())),
      jep::JsonEvent::Number(v)    => JsonEvent::Number(T::from(v.to_string())),
      jep::JsonEvent::Boolean(v)   => JsonEvent::Boolean(*v),
      jep::JsonEvent::Null         => JsonEvent::Null,
      jep::JsonEvent::StartArray   => JsonEvent::StartArray,
      jep::JsonEvent::EndArray     => JsonEvent::EndArray,
      jep::JsonEvent::StartObject  => JsonEvent::StartObject,
      jep::JsonEvent::EndObject    => JsonEvent::EndObject,
      jep::JsonEvent::ObjectKey(v) => JsonEvent::ObjectKey(T::from(v.to_string())),
      jep::JsonEvent::Eof          => JsonEvent::Eof,
    }
  }
}

impl<'a, T> From<json_event_parser::JsonEvent<'_>> for JsonEvent<T>
where T : AsRef<[u8]> + From<String> //+ ToOwned<Owned=T> + for<'b> std::convert::From<&'b std::borrow::Cow<'b, str>>
{
  fn from(jev: json_event_parser::JsonEvent<'_>) -> Self {
    Self::from(&jev)
  }
}

#[test]
fn from_string() {
  let cow = std::borrow::Cow::Borrowed("distring");
  let jev = json_event_parser::JsonEvent::String(cow);
  let ev : JsonEvent<String> = JsonEvent::from(jev);
  // let expected = String::from("distring".as_bytes());
  let expected = String::from("distring");
  assert_eq!(ev, JsonEvent::String(expected))
}

#[test]
fn from_vec() {
  let cow = std::borrow::Cow::Borrowed("distring");
  let jev = json_event_parser::JsonEvent::String(cow);
  let ev : JsonEvent<Vec<u8>> = JsonEvent::from(jev);
  let expected = Vec::from("distring".as_bytes());
  assert_eq!(ev, JsonEvent::String(expected))
}

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
  pub fn next_buf<'a>(&'a self, _buf : &mut Vec<u8>) -> Option<JsonEvent<String>> {
    let mut binding = self.reader.borrow_mut();
    let jep_event_result = binding.read_next_event().expect("TODO fixme");
    let event_result = JsonEvent::from(jep_event_result);
    Some(event_result)
    // eventicize!(self, event_result)
    // // convert to a type with a self-contained buffer
    // if let Some(ref json_event) = eventicize!(self, event_result) {
    //   Some(crate::JsonEvent::from(json_event))
    // } else {
    //   None
    // }
  }
}

impl Iterator for JsonEvents {
  type Item = JsonEvent<String>;

  fn next(&mut self) -> Option<<Self as Iterator>::Item> {
    // read the event which contains a reference to the buffer in self._buf
    match self.reader.borrow_mut().read_next_event() {
        Ok(jev) => Some(JsonEvent::from(jev)),
        Err(err) => {
          fetch_err_context(err, self.reader.borrow_mut().reader());
          None
        }
    }
  }
}
