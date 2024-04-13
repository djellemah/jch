use crate::sendpath::SendPath;
use crate::handler::Handler;
use crate::jsonpath::JsonPath;
use crate::sender::Event;
use crate::sender::Sender;

// for sending the same Path representation over the channel as the one that's constructed
#[allow(unused_macros)]
macro_rules! package_same {
  ($tx:ident,$depth:ident,&$parents:expr) => {
    $tx.send( Some(($depth,$parents.clone())) )
  };
  ($tx:ident,$depth:ident,$parents:expr) => {
    $tx.send(Some(($depth,$parents)))
  };
}

// send a different Path representation over the channel.
#[allow(unused_macros)]
macro_rules! package {
  // see previous to distinguish where clone() is needed
  ($tx:ident,0,&$parents:expr) => {
    $tx.send( &Event::Path(0, SendPath::from($parents)) )
  };
  ($tx:ident,0,$parents:expr) => {
    $tx.send( &Event::Path(0, SendPath::from($parents)) )
  };
  ($tx:ident,$depth:ident,&$parents:expr) => {
    $tx.send( &Event::Path(0, SendPath::from($parents)) )
  };
  ($tx:ident,$depth:ident,$parents:expr) => {
    $tx.send( &Event::Path(0, SendPath::from($parents)) )
  };
}

pub struct Valuer(pub fn(&JsonPath) -> bool);

impl Handler for Valuer
{
  type V<'l> = serde_json::Value;

  fn match_path(&self, path: &JsonPath) -> bool {
    self.0(path)
  }

  // convert the string contained in the JsonEvent into a serde_json::Value
  // and call tx.send with that.
  fn maybe_send_value<'a, Snd>(&self, path : &JsonPath, jev : &json_event_parser::JsonEvent, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  where Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
    use json_event_parser::JsonEvent::*;
    match jev {
      String(v) => if self.match_path(&path) {
        let value = serde_json::Value::String(v.to_string());
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(&Event::Value(SendPath::from(path),value))
      } else {
        // just send the path
        package!(tx,0,path)
      }
      Number(v) => if self.match_path(&path) {
        let value : serde_json::Number = match serde_json::from_str(v) {
            Ok(n) => n,
            Err(msg) => panic!("{v} appears to be not-a-number {msg}"),
        };
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(&Event::Value(SendPath::from(path), serde_json::Value::Number(value)))
      } else {
        // just send the path
        package!(tx,0,path)
      }
      Boolean(v) => if self.match_path(&path) {
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(&Event::Value(SendPath::from(path), serde_json::Value::Bool(*v)))
      } else {
        // just send the path
        package!(tx,0,path)
      },
      Null => if self.match_path(&path) {
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(&Event::Value(SendPath::from(path), serde_json::Value::Null))
      } else {
        // just send the path
        package!(tx,0,path)
      },
      _ => todo!(),
    }
  }
}

pub struct ValueSender;

impl Sender<Event<serde_json::Value>> for ValueSender {
  type SendError = ();

  // Here's where we actually do something with the json event
  // That is, decouple the handling of the parse events, from the actual parsing stream.
  fn send<'a>(&mut self, ev: &'a Event<serde_json::Value>) -> Result<(), Self::SendError> {
    Ok(println!("sent {ev:?}"))
  }
}
