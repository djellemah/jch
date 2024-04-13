struct Plain;

impl Handler for Plain
{
  // default implementation that does nothing and returns OK
  #[allow(unused_variables)]
  fn maybe_send_value<'a, Snd>(&self, path : &JsonPath, ev : &json_event_parser::JsonEvent, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  // see Handler for an explanation of this
  where Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
    println!("{path}");
    Ok(())
  }

  fn match_path(&self, path : &JsonPath) -> bool {
    println!("{path}");
    // ensure all paths are sent
    // if this was true, maybe_send_values would be called with the value as well.
    false
  }
}
