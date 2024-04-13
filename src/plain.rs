use crate::handler::Handler;
use crate::jsonpath::JsonPath;
use crate::sendpath::Event;
use crate::sendpath::Sender;

pub struct Plain;

impl Handler for Plain
{
  type V<'l> = ();

  // default implementation that does nothing and returns OK
  fn maybe_send_value<'a, Snd>(&self, path : &JsonPath, _ev : &json_event_parser::JsonEvent, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  // see Handler for an explanation of this
  where Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
    if self.match_path(path) {
      match tx.send(&Event::Value(path.into(), ())) {
        Ok(()) => println!("{path}"),
        Err(_err) => println!("err"),
    }
    }
    Ok(())
  }

  fn match_path(&self, path : &JsonPath) -> bool {
    println!("match {path}");
    // if this was true, maybe_send_values would be called with the value as well.
    false
  }
}

impl Sender<Event<()>> for Plain {
  type SendError = ();

  // Here's where we actually do something with the json event
  // That is, decouple the handling of the parse events, from the actual parsing stream.
  fn send<'a>(&mut self, ev: &'a Event<()>) -> Result<(), Self::SendError> {
    Ok(println!("sent {ev:?}", ))
  }
}
