//! Mimimal implementation for a Sender to have a function which receives the events.

use crate::sender::Event;
use crate::sender::Sender;

// This is a lot of machinery just to call a function :-\
pub struct FnSnd<SendValue>(pub fn(Event<SendValue>) -> Result<(), Box<dyn std::error::Error>>);

impl<SendValue> Sender<SendValue> for FnSnd<SendValue>
{
  // Here's where we actually do something with the json event
  // That is, decouple the handling of the parse events, from the actual parsing stream.
  fn send(&mut self, ev: crate::sender::Ptr<Event<SendValue>>) -> Result<(), Box<dyn std::error::Error>> {
    self.0(crate::sender::Ptr::<Event<SendValue>>::into_inner(ev).unwrap())
  }
}
