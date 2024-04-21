/*!
The Sender trait.

The Handler will ultimately send this event to an implementation of Sender.

Parameterised because it might need to be sent over a channel, or it might not.
*/

use crate::sendpath::SendPath;

#[allow(dead_code)]
#[derive(Debug,Clone)]
pub enum Event<V> {
  // depth and path
  Path(u64,SendPath),
  // path with the value at that path
  Value(SendPath,V),
  Finished,
  Error(String),
}

/// This can be implemented by anything from a function call to a channel.
pub trait Sender<Event> {
  type SendError : std::fmt::Debug;
  fn send<'a>(&mut self, ev: Box<Event>) -> Result<(), Self::SendError>;
}
