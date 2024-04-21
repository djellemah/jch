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
#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub enum JsonEvent<T> {
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

impl<T : for<'a> From<&'a str>> From<&json_event_parser::JsonEvent<'_>> for JsonEvent<T>{
  fn from(jev: &json_event_parser::JsonEvent<'_>) -> Self {
    match *jev {
      json_event_parser::JsonEvent::String(v) => JsonEvent::String(v.into()),
      json_event_parser::JsonEvent::Number(v) => JsonEvent::Number(v.into()),
      json_event_parser::JsonEvent::Boolean(v) => JsonEvent::Boolean(v.into()),
      json_event_parser::JsonEvent::Null => JsonEvent::Null,
      json_event_parser::JsonEvent::StartArray => JsonEvent::StartArray,
      json_event_parser::JsonEvent::EndArray => JsonEvent::EndArray,
      json_event_parser::JsonEvent::StartObject => JsonEvent::StartObject,
      json_event_parser::JsonEvent::EndObject => JsonEvent::EndObject,
      json_event_parser::JsonEvent::ObjectKey(v) => JsonEvent::ObjectKey(v.into()),
      json_event_parser::JsonEvent::Eof => JsonEvent::Eof,
    }
  }
}

/// Converts json_event_parser events to JsonEvent<String> which contains its own buffer.
pub struct Plain(pub fn(&JsonPath) -> bool);

impl Handler for Plain
{
  type V<'l> = JsonEvent<String>;

  // default implementation that does nothing and returns OK
  fn maybe_send_value<'a, Snd>(&self, path : &JsonPath, ev : &json_event_parser::JsonEvent, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  // see Handler for an explanation of this
  where
    Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
    if self.match_path(path) {
      match tx.send(Box::new(Event::Value(path.into(), JsonEvent::from(ev)))) {
        Ok(()) => (),
        Err(err) => eprintln!("{err:?}"),
      }
    }
    Ok(())
  }

  fn match_path(&self, path : &JsonPath) -> bool {
    self.0(path)
  }
}
