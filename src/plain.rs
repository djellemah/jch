/*!
A really simple visitor that always prints out everything.
*/

use crate::handler::Handler;
use crate::jsonpath::JsonPath;
use crate::sender::Sender;
use crate::sender::Ptr;
use crate::parser::JsonEvent;

/// Converts json_event_parser events to JsonEvent<String> which contains its own buffer.
pub struct Plain(pub fn(&JsonPath) -> bool);

type SendValue = JsonEvent<String>;

impl<'l> Handler<'l, dyn Sender<SendValue> + 'l,SendValue> for Plain
where SendValue: std::fmt::Debug + Clone
{
  fn match_path(&self, path : &JsonPath) -> bool {
    self.0(path)
  }

  /// send the event provided the fn at self.0 returns true
  fn maybe_send_value(&self, path : &JsonPath, ev : crate::sender::Ptr<JsonEvent<String>>, tx : &mut (dyn Sender<SendValue> + 'l))
  -> Result<(),Box<dyn std::error::Error>>
  {
    if self.match_path(path) {
      use crate::sender::Event;
      tx
        // NOTE ev is an Arc
        .send(Ptr::new(Event::Value(path.into(), ev.clone())))
        .unwrap_or_else(|err| eprintln!("error sending {ev:?} because {err:?}"))
    }
    // ;
    Ok(())
  }
}
