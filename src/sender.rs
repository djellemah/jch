/*!
The Sender trait.

The Handler will ultimately send this event to an implementation of Sender.

Parameterised because it might need to be sent over a channel, or it might not.
*/

use crate::sendpath::SendPath;

/// SendValue is intended to be some kind of value - ie String, Number, Bool, Null etc. But it could be anything.
/// In the most general sense, it's the value identified by a particular Path.
#[derive(Debug,Clone)]
pub enum Event<SendValue>{
  // depth and path
  Path(u64,SendPath),
  // path with the value at that path
  Value(SendPath,SendValue),
  Finished,
  Error(SendPath,String),
}

/// This can be implemented by anything from a function call to a channel.
pub trait Sender<SendValue>
{
  fn send(&mut self, ev: Box<Event<SendValue>>) -> Result<(), Box<dyn std::error::Error>>;
}
