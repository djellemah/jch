//! Send parse events across a channel, to decouple parsing from handling.

use crate::parser::JsonEvents;
use crate::jsonpath::JsonPath;
use crate::sender::Sender;
use crate::sender::Event;

pub trait Producer<T,E : std::error::Error> {
  fn send(&mut self, a: T) -> Result<(),E>;
}

pub trait Consumer<T,E : std::error::Error + ?Sized> {
  fn recv(&mut self) -> Result<T,Box<dyn std::error::Error>>;
}

// implementation of Producer and Consumer for rtrb ring buffer
pub mod rb {
  pub struct RbProducer<T>(pub rtrb::Producer<T>);

  impl<T> super::Producer<T, rtrb::PushError<T>> for RbProducer<T> {
    fn send(&mut self, arg : T) -> Result<(), rtrb::PushError<T>> {
      match self.0.push(arg) {
        Ok(()) => Ok(()),
        Err(rtrb::PushError::Full(rejected)) if !self.0.is_abandoned() => {
          // ringbuffer is full, so wait for signal from consumer
          std::thread::park();
          // recurse instead of loop - to avoid borrow and Rc in Self
          // recursion will not be deep - because of park
          self.send(rejected)
        }
        // ringbuffer is closed, or something else went wrong
        err => err
      }
    }
  }

  pub struct RbConsumer<T>(pub rtrb::Consumer<T>, pub std::thread::Thread);

  impl<T,E : std::error::Error + ?Sized> super::Consumer<T,E> for RbConsumer<T> {
    fn recv(&mut self) -> Result<T, Box<dyn std::error::Error>> {
      loop {
        match self.0.pop() {
          Ok(jev) => return Ok(jev),
          Err(rtrb::PopError::Empty) if !self.0.is_abandoned() => {
            // tell the producer to carry on
            self.1.unpark();
            continue
          },
          Err(err) => return Err(Box::new(err))
        }
      }
    }
  }

  impl<T : Clone + std::fmt::Debug + 'static> super::Sender<T> for RbProducer<super::Event<T>> {
    // Here's where we actually do something with the json event
    // That is, decouple the handling of the parse events, from the actual parsing stream.
    fn send(&mut self, ev: Box<crate::sender::Event<T>>) -> Result<(), Box<dyn std::error::Error>> {
      // wrangle rtrb::PushError into std::error::Error
      Ok(super::Producer::send(self, *ev)?)
    }
  }
}

// implementation of Producer and Consumer for crossbeam::channel
pub mod ch {
  use super::Event;

  pub struct ChSender<T>(pub crossbeam::channel::Sender<Event<T>>);

  impl<T,E : std::error::Error> super::Producer<T, E> for ChSender<T> {
    fn send(&mut self, _: T) -> Result<(), E> { todo!() }
  }

  impl<T : Clone + std::fmt::Debug + std::marker::Send + 'static> crate::channel::Sender<T> for ChSender<T> {
    // Convert a Json
    fn send(&mut self, ev: Box<crate::sender::Event<T>>) -> Result<(), Box<dyn std::error::Error>> {
      // wrangle crossbeam::channel::SendError into std::error::Error
      Ok(self.0.send(*ev)?)
    }
  }
}

pub fn channels(jev : &mut dyn JsonEvents<String>) {
  // this seems to be about optimal wrt performance
  const RING_BUFFER_BOUND : usize = (2usize).pow(21); // 8192

  // Events in the RingBuffer contains whatever Valuer is sending, so JsonEvent<String>
  type SendValue = serde_json::Value;

  let (tx, rx) = rtrb::RingBuffer::<Event<SendValue>>::new(RING_BUFFER_BOUND);
  let (mut tx, mut rx) = (rb::RbProducer(tx), rb::RbConsumer(rx,std::thread::current()));

  // consumer thread
  let cons_thr = std::thread::spawn(move || {
    let rx = &mut rx as &mut dyn Consumer<Event<SendValue>, dyn std::error::Error>;
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
    use crate::handler::Handler;
    let visitor = crate::valuer::Valuer(|_| true);
    let tx = &mut tx as &mut dyn crate::sender::Sender<SendValue>;
    visitor.value(jev, JsonPath::new(), 0, tx).unwrap_or_else(|_| println!("uhoh"));
  }

  cons_thr.join().unwrap();
}
