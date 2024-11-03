/*!
The Sender trait.

The Handler will ultimately send this event to an implementation of Sender.

Parameterised because it might need to be sent over a channel, or it might not.
*/

use crate::sendpath::SendPath;

/// Just a newtype struct so we can send values with no Rc, no Arc, no Box etc
#[derive(Debug)]
pub struct NonWrap<T>(T);

// Hmm. This has a ?Sized  in Arc implementation
impl<T> std::ops::Deref for NonWrap<T> {
  type Target = T;

  #[inline]
  fn deref(&self) -> &T { &self.0 }
}

impl<T> From<T> for NonWrap<T> {
  #[inline]
  fn from(t: T) -> Self { NonWrap(t) }
}

impl<T> AsRef<T> for NonWrap<T> {
  #[inline]
  fn as_ref(&self) -> &T { &self.0 }
}

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
///
/// Item is usually Event<JsonEvent<String>>
/// So Wrapper could be
/// Rc<Event<SendValue>>
/// Arc<Event<SendValue>>
/// Box<Event<SendValue>>
/// NonWrap<Event<SendValue>>
//
// Both type parameters are necessary here, and the method cannot be
// parameterised, otherwise the trait becomes not dyn-compatible, because rust
// can't build a vtable for a parameterised method.
pub trait Sender<Item, Wrapper : Send + std::ops::Deref<Target = Item>>
{
  fn send(&mut self, ev: Wrapper) -> Result<(), Box<dyn std::error::Error>>;
}
