use crate::jsonpath::JsonPath;
use crate::sender::Sender;
use crate::sender::Event;

pub struct ChSender<T>(std::sync::mpsc::SyncSender<Event<T>>);

impl<T : Clone> Sender<Event<T>> for ChSender<T> {
  type SendError=std::sync::mpsc::SendError<Event<T>>;

  // Here's where we actually do something with the json event
  // That is, decouple the handling of the parse events, from the actual parsing stream.
  fn send<'a>(&mut self, ev: &'a Event<T>) -> Result<(), Self::SendError> {
    self.0.send(ev.clone())
  }
}

// T = serde_json::Value, for example
pub fn channels(jev : &mut super::JsonEvents) {
  // let (tx, rx) = std::sync::mpsc::sync_channel::<Event>(4096);
  // this seems to be about optimal wrt performance
  let (tx, rx) = std::sync::mpsc::sync_channel::<Event<serde_json::Value>>(8192);
  // let (tx, rx) = std::sync::mpsc::sync_channel::<Event>(16384);
  // let (tx, rx) = std::sync::mpsc::sync_channel::<Event>(32768);

  // consumer thread
  let cons_thr = std::thread::spawn(move || {
    while let Ok(event) = rx.recv() {
      match event  {
        Event::Path(depth,path) => println!("{depth}:{}", path),
        Event::Finished => {println!("Event::Finished"); break},
        Event::Value(p,v) => println!("{p} => {v}"),
        Event::Error(_) => todo!(),
      }
    }
  });

  {
    // jump through hoops so cons_thr join will work
    let tx = tx.clone();
    // wrap tx in a thing that implements Sender
    let mut tx_sender: ChSender<serde_json::Value> = ChSender(tx);
    use crate::handler::Handler;
    let visitor = crate::valuer::Valuer(|_| true);
    visitor.value(jev, JsonPath::new(), 0, &mut tx_sender).unwrap_or_else(|_| println!("uhoh"));
    // inner tx dropped automatically here
  }
  // done with the weird hoops
  drop(tx);
  cons_thr.join().unwrap();
}
