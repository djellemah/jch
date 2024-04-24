/*!
The interface to the parser, currently json-event-parser.
*/

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

// Source of Json parse events, ie the json parser
pub struct JsonEvents(json_event_parser::FromReadJsonReader<Box<dyn std::io::BufRead>>);

impl JsonEvents {
  pub fn new(istream : Box<dyn std::io::BufRead>) -> Self {
    Self(json_event_parser::FromReadJsonReader::new(istream))
  }

  pub fn next_buf<'a>(&'a mut self) -> Option<JsonEvent<String>> {
    match self.0.read_next_event() {
      Ok(ref jep_event) => Some(JsonEvent::from(jep_event)),
      Err(err) => {
        eprintln!("{err:?}");
        None
      }
    }
  }
}

impl Iterator for JsonEvents {
  type Item = JsonEvent<String>;

  fn next(&mut self) -> Option<<Self as Iterator>::Item> {
    // read the event which contains a reference to the buffer in self._buf
    match self.0.read_next_event() {
      Ok(jev) => Some(JsonEvent::from(jev)),
      Err(err) => {
        eprintln!("{err:?}");
        None
      }
    }
  }
}
