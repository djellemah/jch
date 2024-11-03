/*!
A really simple visitor that always prints out everything.
*/

use crate::handler::Handler;
use crate::jsonpath::JsonPath;
use crate::sender::Sender;
use crate::sender;
use crate::sender::Event;
use crate::parser::JsonEvent;

/// Converts json_event_parser events to JsonEvent<String> which contains its own buffer.
pub struct Plain<SendWrapper>(pub fn(&JsonPath) -> bool, pub std::marker::PhantomData<SendWrapper>);

type SendValue = JsonEvent<String>;
type SendEvent = Event<SendValue>;

impl<'l, SendWrapper: 'l> Handler<'l,
  SendValue,
  SendWrapper,
  dyn sender::Sender< SendEvent, SendWrapper > + 'l,
> for Plain<SendWrapper>
where SendWrapper : Send + std::ops::Deref<Target=sender::Event<SendValue>> + From<sender::Event<SendValue>>
{
  fn match_path(&self, path : &JsonPath) -> bool {
    self.0(path)
  }

  /// send the event provided the fn at self.0 returns true
  fn maybe_send_value(&self, path : &JsonPath, ev : JsonEvent<String>, tx : &mut (dyn Sender<Event<SendValue>,SendWrapper> + 'l))
  -> Result<(),Box<dyn std::error::Error>>
  {
    if self.match_path(path) {
      use crate::sender::Event;
      tx
        .send(Event::Value(path.into(), ev.clone()).into())
        .unwrap_or_else(|err| eprintln!("error sending {ev:?} because {err:?}"))
    };
    Ok(())
  }
}
