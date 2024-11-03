//! Mimimal implementation for a Sender to have a function which receives the events.

use crate::sender;
use crate::sender::Event;
use crate::sender::Sender;

// This is a lot of machinery just to call a function :-\
#[allow(clippy::type_complexity)] // oh come on
pub struct FnSnd<SendValue>(pub fn(&Event<SendValue>) -> Result<(), Box<dyn std::error::Error>>);

impl<SendValue: Send> Sender<Event<SendValue>,sender::NonWrap<Event<SendValue>>> for FnSnd<SendValue>
{
  // Here's where we actually do something with the json event
  // That is, decouple the handling of the parse events, from the actual parsing stream.
  fn send(&mut self, ev: sender::NonWrap<Event<SendValue>>) -> Result<(), Box<dyn std::error::Error>> {
    self.0(ev.as_ref())
  }
}
