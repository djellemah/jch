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
    Error{line : u64, col : u64, message: T},
}

impl<T> std::fmt::Display for JsonEvent<T>
where T : std::convert::AsRef<[u8]> + std::fmt::Debug + std::fmt::Display
{
  fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    if let Self::Error{line, col, message} = &self {
      write!(fmt, "error at {}:{}: {message}", line+1, col+1) // because line and col numbers from the parser are 0-based
    } else {
      write!(fmt, "{self:?}")
    }
  }
}

#[test]
fn create() {
  let _je = JsonEvent::String("hello".to_string());
  let _je = JsonEvent::Number("5".to_string());
  let _je = JsonEvent::ObjectKey("cle".to_string());
}

impl<T> From<&json_event_parser::JsonEvent<'_>> for JsonEvent<T>
where T : AsRef<[u8]> + From<String> // because we want storage + conversion from Cow<'_,str>
{
  fn from(jev: &json_event_parser::JsonEvent<'_>) -> Self {
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

impl<T> From<json_event_parser::JsonEvent<'_>> for JsonEvent<T>
where T : AsRef<[u8]> + From<String> //+ ToOwned<Owned=T> + for<'b> std::convert::From<&'b std::borrow::Cow<'b, str>>
{
  fn from(jev: json_event_parser::JsonEvent<'_>) -> Self {
    Self::from(&jev)
  }
}

#[test]
fn from_string() {
  let cow = std::borrow::Cow::from("distring");
  let jev = json_event_parser::JsonEvent::String(cow);
  let ev : JsonEvent<String> = JsonEvent::from(jev);
  let expected = String::from("distring");
  assert_eq!(ev, JsonEvent::String(expected))
}

#[test]
fn from_vec() {
  let cow = std::borrow::Cow::from("distring");
  let jev = json_event_parser::JsonEvent::String(cow);
  let ev : JsonEvent<Vec<u8>> = JsonEvent::from(jev);
  let expected = Vec::from("distring".as_bytes());
  assert_eq!(ev, JsonEvent::String(expected))
}

/// Source of Json parse events, ie the json parser
/// All interfaces with the parser must happen through this.
pub trait JsonEvents<'l, Stringish>
where Stringish : 'l + AsRef<[u8]> + From<String> // because we want storage + conversion from Cow<'_,str>
{
   fn next_event(&mut self) -> Result<JsonEvent<Stringish>, Box<dyn std::error::Error>>;
}

/// Source of json events from json_event_parser
pub struct JsonEventParser(json_event_parser::FromReadJsonReader<Box<dyn std::io::BufRead>>);

impl JsonEventParser {
  pub fn new(istream : Box<dyn std::io::BufRead>) -> Self {
    Self(json_event_parser::FromReadJsonReader::new(istream))
  }
}

impl<'l, Stringish> JsonEvents<'l, Stringish> for JsonEventParser
where
Stringish : std::convert::AsRef<[u8]> + std::convert::From<std::string::String> + 'l
{
  fn next_event<'a>(&'a mut self) -> Result<JsonEvent<Stringish>, Box<(dyn std::error::Error)>> {
    use json_event_parser::ParseError;
    match self.0.read_next_event() {
      Ok(ref jep_event) => Ok(JsonEvent::from(jep_event)),
      Err(ParseError::Io(err)) => {
        Err(format!("{err:?}").into())
      }
      Err(ParseError::Syntax(syntax_error)) => {
        use std::ops::Range;
        // use json_event_parser::SyntaxError;
        use json_event_parser::TextPosition;

        // can't match because private fields
        // json_event_parser::SyntaxError{location, message}
        let Range{start, ..} : Range<TextPosition> = syntax_error.location();
        let msg : String = syntax_error.message().into();
        Ok(JsonEvent::Error{line : start.line, col : start.column, message: msg.into()})
      }
    }
  }
}

impl Iterator for JsonEventParser {
  type Item = JsonEvent<String>;

  fn next(&mut self) -> Option<<Self as Iterator>::Item> {
    match self.next_event() {
     Ok(jev) => Some(jev),
     err => panic!("{err:?}"),
    }
  }
}
