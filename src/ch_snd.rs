use super::JsonPath;
use super::Event;

pub type SendError<T> = std::sync::mpsc::SendError<T>;
pub type SendResult<T> = Result<(), SendError<T>>;
pub struct ChSender<T>(std::sync::mpsc::SyncSender<T>);

impl<T> super::Sender<T> for ChSender<T> {
  type SendError=std::sync::mpsc::SendError<T>;

  fn send(&self, t : T) -> SendResult<T> {
    self.0.send(t)
  }
}

pub fn channels(jev : &mut super::JsonEvents) {
  // let (tx, rx) = std::sync::mpsc::sync_channel::<Event>(4096);
  // this seems to be about optimal wrt performance
  let (tx, rx) = std::sync::mpsc::sync_channel::<Event>(8192);
  // let (tx, rx) = std::sync::mpsc::sync_channel::<Event>(16384);
  // let (tx, rx) = std::sync::mpsc::sync_channel::<Event>(32768);

  // consumer thread
  std::thread::spawn(move || {
    loop {
      match rx.recv() {
        Ok(Event::Path(depth,path)) => println!("{depth}:{}", path),
        Ok(Event::Finished) => break,
        Ok(Event::Value(p,v)) => println!("{p} => {v}"),
        Err(err) => { eprintln!("ending consumer: {err}"); break },
      }
    }
  });

  // wrap tx in a thing that implements Sender
  let tx = ChSender(tx);
  use super::Handler;
  let visitor = super::Plain;
  // producer loop pass the event source (jev) to the
  loop {
    match visitor.find_path::<ChSender<Event>>(jev, JsonPath::new(), 0, &tx ) {
      Ok(()) => (),
      Err(err) => { eprintln!("ending producer {err}"); break },
    }
  }
}
