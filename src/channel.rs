//! Send parse events across a channel or ringbuffer, to decouple parsing from handling.

use std::sync::Arc;

use crate::jsonpath::JsonPath;
use crate::parser::JsonEventSource;
use crate::sender::Event;
use crate::sender;

pub trait Producer<T,E : std::error::Error> {
  fn send(&mut self, a: T) -> Result<(),E>;
}

pub trait Consumer<T,E : std::error::Error + ?Sized> {
  fn recv(&mut self) -> Result<T,Box<dyn std::error::Error>>;
}

// implementation of Producer and Consumer for rtrb ring buffer
pub mod rb {
  use std::sync::Arc;
  use super::Event;

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

  // Second parameter is the producer thread, which we use for park/unpark
  // when ringbuffer is full.
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

  use crate::sender;
  /// To accept events from Handler
  impl<T : Clone + std::fmt::Debug + 'static + Send + Sync> sender::Sender<Event<T>,Arc<Event<T>>> for RbProducer<super::Event<T>> {
    // Here's where we actually do something with the json event
    // That is, decouple the handling of the parse events, from the actual parsing stream.
    fn send(&mut self, ev: Arc<crate::sender::Event<T>>) -> Result<(), Box<dyn std::error::Error>> {
      // wrangle rtrb::PushError into std::error::Error
      Ok(super::Producer::send(self, Arc::<Event<T>>::into_inner(ev).unwrap())?)
    }
  }
}

// implementation of Consumer and Sender for crossbeam::channel
pub mod ch {
  use std::ops::Deref;

  use super::Event;
  use crate::sender;

  impl<T,E : std::error::Error + ?Sized> super::Consumer<T,E> for crossbeam::channel::Receiver<T> {
    fn recv(&mut self) -> Result<T, Box<dyn std::error::Error>> {
      Ok(crossbeam::channel::Receiver::recv(self)?)
    }
  }

  // For some wrapper (incl Arc and Rc)
  impl<T,W> sender::Sender<Event<T>,W>
  for crossbeam::channel::Sender<crate::channel::Event<T>>
  where
    T: Send + 'static,
    W : Deref<Target=Event<T>> + Send + Into<Event<T>>
  {
    fn send(&mut self, ev: W) -> Result<(), Box<dyn std::error::Error>> {
      Ok(crossbeam::channel::Sender::send(self, ev.into())?)
    }
  }


  // For no wrapper
  impl<T: Send + Sync + 'static>
  sender::Sender<Event<T>,sender::NonWrap<Event<T>>>
  for crossbeam::channel::Sender<sender::NonWrap<crate::channel::Event<T>>> {
    fn send(&mut self, ev: sender::NonWrap<Event<T>>) -> Result<(), Box<dyn std::error::Error>> {
      Ok(crossbeam::channel::Sender::send(self, ev)?)
    }
  }
}

pub fn ringbuffer(jev : &mut dyn JsonEventSource<String>) {
  // this seems to be about optimal wrt performance
  const RING_BUFFER_BOUND : usize = (2usize).pow(21); // 8192

  // Events in the RingBuffer contains whatever Valuer is sending, so JsonEvent<String>
  type SendValue = serde_json::Value;

  let (tx, rx) = rtrb::RingBuffer::<Event<SendValue>>::new(RING_BUFFER_BOUND);
  // wrap of these is required, so we can get to the thread to park/unpark
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
    visitor.value(jev, JsonPath::new(), 0, &mut tx as &mut dyn sender::Sender<Event<SendValue>, Arc<Event<SendValue>>>).unwrap_or_else(|_| println!("uhoh"));
  }

  cons_thr.join().unwrap();
}

impl<T> From<Arc<Event<T>>> for Event<T> {
  fn from(value: Arc<Event<T>>) -> Self {
    Arc::<Event<T>>::into_inner(value).expect("There must be a strong reference here, otherwise something else broke")
  }
}

pub fn channels(jev : &mut dyn JsonEventSource<String>) {
  // this seems to be about optimal wrt performance
  const CHANNEL_SIZE : usize = 8192;

  // Events in the RingBuffer contains whatever Valuer is sending, so JsonEvent<String>
  type SendValue = serde_json::Value;

  let (mut tx, mut rx) = crossbeam::channel::bounded::<Event<SendValue>>(CHANNEL_SIZE);

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
    let tx = &mut tx as &mut dyn sender::Sender<Event<SendValue>,Arc<Event<SendValue>>>;
    visitor.value(jev, JsonPath::new(), 0, tx).unwrap_or_else(|_| println!("uhoh"));
  }

  cons_thr.join().unwrap();
}
