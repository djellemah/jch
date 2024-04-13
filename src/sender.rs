// The Sender trait.
// 
// The Handler will ultimately send this event to an implementation of Sender.

use crate::sendpath::SendPath;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Event<V> {
  Path(u64,SendPath),
  Value(SendPath,V),
  Finished,
  Error(String),
}

// This can be anything from a function call to a channel.
pub trait Sender<Event> {
  type SendError;
  fn send<'a>(&mut self, ev: &'a Event) -> Result<(), Self::SendError>;
}
