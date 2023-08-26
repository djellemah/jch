struct Valuer(fn(&JsonPath) -> bool);

impl Handler for Valuer
{
  fn match_path(&self, path: &JsonPath) -> bool {
    self.0(path)
  }

  fn maybe_send_value<V,Snd>(&self, path : &JsonPath, &ev : &json_event_parser::JsonEvent, tx : &mut Snd)
  -> Result<(),Snd::SendError>
  where Snd : Sender<Event<V>>, V : &[u8]
  {
    use json_event_parser::JsonEvent::*;
    match ev {
      String(v) => if self.match_path(&path) {
        let value = serde_json::Value::String(v.into());
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(&Event::Value(SendPath::from(path),value))
      } else {
        // just send the path
        package!(tx,0,path)
      }
      Number(v) => if self.match_path(&path) {
        let value : serde_json::Number = match serde_json::from_str(v) {
            Ok(n) => n,
            Err(msg) => panic!("{v} appears to be not-a-number {msg}"),
        };
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(&Event::Value(SendPath::from(path), serde_json::Value::Number(value)))
      } else {
        // just send the path
        package!(tx,0,path)
      }
      Boolean(v) => if self.match_path(&path) {
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(&Event::Value(SendPath::from(path), serde_json::Value::Bool(v)))
      } else {
        // just send the path
        package!(tx,0,path)
      },
      Null => if self.match_path(&path) {
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(&Event::Value(SendPath::from(path), serde_json::Value::Null))
      } else {
        // just send the path
        package!(tx,0,path)
      },
      _ => todo!(),
    }
  }
}
