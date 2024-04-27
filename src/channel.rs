//! Send parse events across a channel, to decouple parsing from handling.

use crate::parser::JsonEvents;
use crate::jsonpath::JsonPath;
use crate::sender::Sender;
use crate::sender::Event;

pub struct ChSender<T>(pub crossbeam::channel::Sender<Event<T>>);

impl<T : Clone + std::fmt::Debug> Sender<Event<T>> for ChSender<T> {
  type SendError=crossbeam::channel::SendError<Event<T>>;

  // Here's where we actually do something with the json event
  // That is, decouple the handling of the parse events, from the actual parsing stream.
  fn send<'a>(&mut self, ev: Box<Event<T>>) -> Result<(), Self::SendError> {
    self.0.send(*ev)
  }
}

// T = serde_json::Value, for example
pub fn channels(jev : &mut dyn JsonEvents<String>) {
  // this seems to be about optimal wrt performance
  const CHANNEL_SIZE : usize = 8192;
  let (tx, rx) = crossbeam::channel::bounded(CHANNEL_SIZE);

  // consumer thread
  let cons_thr = std::thread::spawn(move || {
    while let Ok(event) = rx.recv() {
      match event  {
        Event::Path(depth,path) => println!("{depth}:{}", path),
        Event::Value(p,v) => println!("{p} => {v}"),
        Event::Error(p,err) => println!("Event::Error {err} at path '{p}'"),
        Event::Finished => {println!("Event::Finished"); break},
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
