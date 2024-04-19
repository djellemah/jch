//! Mimimal implementation for a Sender to have a function which receives the events.
use crate::sender::Sender;

// This is a lot of machinery just to call a function :-\
pub struct FnSnd<Event,Error>(pub fn(Event) -> Result<(), Error>);

impl<Event,ErrorType> Sender<Event> for FnSnd<Event,ErrorType> {
  type SendError = ErrorType;

  // Here's where we actually do something with the json event
  // That is, decouple the handling of the parse events, from the actual parsing stream.
  fn send(&mut self, ev: Box<Event>) -> Result<(), Self::SendError> {
    self.0(*ev)
  }
}
