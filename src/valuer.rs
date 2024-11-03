//! Converts incoming JsonEvents to serde_json::Value

use crate::handler::Handler;
use crate::jsonpath::JsonPath;
use crate::sender;
use crate::sender::Event;
use crate::sendpath::SendPath;
use crate::parser::JsonEvent;

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
    $tx.send( SendWrapper::from(Event::Path(0, SendPath::from($parents))) )
  };
  ($tx:ident,$depth:ident,&$parents:expr) => {
    $tx.send( &Event::Path(0, SendPath::from($parents)) )
  };
  ($tx:ident,$depth:ident,$parents:expr) => {
    $tx.send( &Event::Path(0, SendPath::from($parents)) )
  };
}

/// Converts json events from the parser to serde_json, and then calls the function.
/// It's implements both the Handler and the Sender, so it sends to itself via a function call.
pub struct Valuer(pub fn(&JsonPath) -> bool);

type SendValue = serde_json::Value;

impl<'l, SendValue, SendWrapper> Handler<'l, SendValue, SendWrapper, (dyn sender::Sender<Event<SendValue>, SendWrapper> + 'l)>
for Valuer
where
  SendValue : 'l + From<serde_json::Value>,
  SendWrapper : 'l + Send + From<Event<SendValue>> + std::ops::Deref<Target=Event<SendValue>>,
{
  fn match_path(&self, path: &JsonPath) -> bool {
    self.0(path)
  }

  // convert the string contained in the JsonEvent into a serde_json::Value
  // and call tx.send with that.
  fn maybe_send_value(&self, path : &JsonPath, jev : JsonEvent<String>, tx : &mut (dyn sender::Sender<Event<SendValue>, SendWrapper> + 'l))
  -> Result<(),Box<dyn std::error::Error>>
  {
    use JsonEvent::*;
    // stop early, ie higher up the tree, if we can.
    if !<Valuer as Handler<'_, SendValue, SendWrapper, dyn sender::Sender<Event<SendValue>, SendWrapper>>>::match_path(self, path) {
      return package!(tx,0,path)
    }

    // otherwise traverse the tree (that is, process events having a longer path with a matching prefix)
    match jev {
      String(v) => {
        let value = serde_json::Value::String(v.to_string());
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(SendWrapper::from(Event::Value(SendPath::from(path),value.into())))
      }
      Number(v) => {
        let value : serde_json::Number = match serde_json::from_str(&v) {
            Ok(n) => n,
            Err(msg) => panic!("{v} appears to be not-a-number {msg}"),
        };
        tx.send(SendWrapper::from(Event::Value(SendPath::from(path), serde_json::Value::Number(value).into())))
      }
      Boolean(v) => {
        tx.send(SendWrapper::from(Event::Value(SendPath::from(path), serde_json::Value::Bool(v).into())))
      }
      Null => {
        tx.send(SendWrapper::from(Event::Value(SendPath::from(path), serde_json::Value::Null.into())))
      }
      // should never receive these. TODO but that fact should be encoded in types.
      StartArray => todo!(),
      EndArray => todo!(),
      StartObject => todo!(),
      EndObject => todo!(),
      ObjectKey(_) => todo!(),
      Eof => todo!(),
      err@ Error{..} => todo!("{err:?}"),
    }
  }
}

pub struct ValueSender<SendWrapper>(std::marker::PhantomData<SendWrapper>);

impl<SendWrapper> sender::Sender<Event<SendValue>,SendWrapper> for ValueSender<SendWrapper>
where SendWrapper : Send + std::fmt::Debug + std::ops::Deref<Target = Event<SendValue>>
{
  // Here's where we actually do something with the json event
  // That is, decouple the handling of the parse events, from the actual parsing stream.
  #[allow(clippy::unit_arg)]
  fn send<'a>(&mut self, ev: SendWrapper) -> Result<(), Box<dyn std::error::Error>> {
    Ok(println!("sent {ev:?}"))
  }
}
