/*!
A really simple visitor that always prints out everything.
*/

use crate::handler::Handler;
use crate::jsonpath::JsonPath;
use crate::sender::Event;
use crate::sender::Sender;
use crate::parser::JsonEvent;

/// Converts json_event_parser events to JsonEvent<String> which contains its own buffer.
pub struct Plain(pub fn(&JsonPath) -> bool);

impl Handler for Plain
{
  type V<'l> = JsonEvent<String>;

  /// send the event provided the fn at self.0 returns true
  fn maybe_send_value<'a, Snd>(&self, path : &JsonPath, ev : &JsonEvent<String>, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  where
    Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
    if self.match_path(path) {
      tx
        .send(Box::new(Event::Value(path.into(), ev.clone())))
        .unwrap_or_else(|err| eprintln!("error sending {ev:?} because {err:?}"))
    }
    Ok(())
  }

  fn match_path(&self, path : &JsonPath) -> bool {
    self.0(path)
  }
}
