struct Plain;

impl Handler for Plain
{
  // default implementation that does nothing and returns OK
  #[allow(unused_variables)]
  fn maybe_send_value<serde_json::Value, Snd : Sender<Event<serde_json::Value>>>(&self, path : &JsonPath, ev : &json_event_parser::JsonEvent, tx : &mut Snd)
  -> Result<(),Snd::SendError> {
    println!("{path}");
    Ok(())
  }

  fn match_path(&self, _path : &JsonPath) -> bool {
    println!("{_path}");
    // ensure all paths are sent
    // if this was true, maybe_send_values would be called with the value as well.
    false
  }
}
