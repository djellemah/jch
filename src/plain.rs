/*!
A really simple visitor that always prints out everything.
*/

use std::fmt::Debug;

use crate::handler::Handler;
use crate::jsonpath::JsonPath;
use crate::sender::Event;
use crate::sender::Sender;

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

/// Converts json_event_parser events to JsonEvent<String> which contains its own buffer.
pub struct Plain(pub fn(&JsonPath) -> bool);

impl Handler for Plain
{
  type V<'l> = JsonEvent<String>;

  /// send the event provided the fn at self.0 returns true
  fn maybe_send_value<'a, Snd>(&self, path : &JsonPath, ev : &JsonEvent<String>, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  where
    Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
    if self.match_path(path) {
      tx
        .send(Box::new(Event::Value(path.into(), ev.clone())))
        .unwrap_or_else(|err| eprintln!("error sending {ev:?} because {err:?}"))
    }
    Ok(())
  }

  fn match_path(&self, path : &JsonPath) -> bool {
    self.0(path)
  }
}
