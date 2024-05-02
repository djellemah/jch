//! Send parse events across a channel, to decouple parsing from handling.

use crate::parser::JsonEvents;
use crate::jsonpath::JsonPath;
use crate::sender::Sender;
use crate::sender::Event;

trait Producer<T,E> {
  fn send(&mut self, a: T) -> Result<(),E>;
}

trait Consumer<T> {
  fn recv(&mut self) -> Result<T,()>;
}

struct RbProducer<T>(rtrb::Producer<T>);

impl<T> Producer<T, rtrb::PushError<T>> for RbProducer<T> {
  fn send(&mut self, arg : T) -> Result<(), rtrb::PushError<T>> {
    match self.0.push(arg) {
      Ok(()) => Ok(()),
      // TODO matching is all messed up here
      err @ Err(rtrb::PushError::Full(_)) => {
        // return false tells rapidjson to stop parsing
        if self.0.is_abandoned() { err }
        else {
          // otherwise ringbuffer is full, so wait for signal from consumer
          std::thread::park();
          if let Err(rtrb::PushError::Full(rejected)) = err {
            // recurse instead of loop - to avoid borrow and Rc in Self
            self.send(rejected)
          } else {
            panic!("How can this not be PushError?")
          }
        }
      }
    }
  }
}

struct RbConsumer<T>(rtrb::Consumer<T>, std::thread::Thread);

impl<T> Consumer<T> for RbConsumer<T> {
  fn recv(&mut self) -> Result<T, ()> {
    while !self.0.is_abandoned() {
      match self.0.pop() {
        Ok(jev) => return Ok(jev),
        Err(rtrb::PopError::Empty) => {
          // tell the producer to carry on
          self.1.unpark();
          continue
        },
      }
    };
    // real PITA to get a type constructor in here
    Err(())
  }
}

impl<T : Clone + std::fmt::Debug> Sender<T> for RbProducer<T> {
  type SendError=rtrb::PushError<T>;

  // Here's where we actually do something with the json event
  // That is, decouple the handling of the parse events, from the actual parsing stream.
  fn send(&mut self, ev: Box<T>) -> Result<(), Self::SendError> {
    Producer::send(self, *ev)
  }
}

pub struct ChSender<T>(pub crossbeam::channel::Sender<Event<T>>);

impl<T : Clone + std::fmt::Debug> Sender<Event<T>> for ChSender<T> {
  type SendError=crossbeam::channel::SendError<Event<T>>;

  // Here's where we actually do something with the json event
  // That is, decouple the handling of the parse events, from the actual parsing stream.
  fn send<'a>(&mut self, ev: Box<Event<T>>) -> Result<(), Self::SendError> {
    self.0.send(*ev)
  }
}

pub fn channels(jev : &mut dyn JsonEvents<String>) {
  // this seems to be about optimal wrt performance
  const RING_BUFFER_BOUND : usize = (2usize).pow(21); // 8192
  let (tx, rx) = rtrb::RingBuffer::new(RING_BUFFER_BOUND);
  let (tx, mut rx) = (RbProducer(tx), RbConsumer(rx,std::thread::current()));
  // consumer thread
  let cons_thr = std::thread::spawn(move || {
    while let Ok(event) = <RbConsumer<Event<_>> as Consumer<Event<_>>>::recv(&mut rx) {
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
    // let tx = tx.clone();
    // wrap tx in a thing that implements Sender
    let mut tx_sender = tx;
    use crate::handler::Handler;
    let visitor = crate::valuer::Valuer(|_| true);
    visitor.value(jev, JsonPath::new(), 0, &mut tx_sender).unwrap_or_else(|_| println!("uhoh"));
    // inner tx dropped automatically here
  }
  // done with the weird hoops
  // drop(tx);
  cons_thr.join().unwrap();
}
