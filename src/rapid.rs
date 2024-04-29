// Because the rapidjson handler needs PascalCase method names.
#![allow(non_snake_case)]

use std::ffi::c_char;

pub struct RustStream {
  reader : Box<dyn std::io::BufRead>,
  peeked : Option<c_char>,
  count : usize,
}

impl RustStream {
  fn new(reader : Box<dyn std::io::BufRead>) -> Self {
    Self{reader, peeked: None, count: 0}
  }

  #[inline]
  fn Peek(self : &mut RustStream) -> c_char {
    if let Some(peeked) = self.peeked {
      // we already have a peek
      peeked
    } else {
      // otherwise fetch it, stash it, and return it
      // TODO inefficient
      let mut buf = [0u8];
      if let Ok(()) = self.reader.read_exact(&mut buf) {
        self.count += 1;
        let rv = buf[0] as c_char;
        self.peeked = Some(rv);
        rv as c_char
      } else {
        // RapidJSON seems to interpret this as eof
        0 as c_char
      }
    }
  }

  #[inline]
  fn Take(self : &mut RustStream) -> c_char {
    if let Some(peeked) = self.peeked {
      // consume peeked first
      let rv = peeked as c_char;
      self.peeked = None;
      rv
    } else {
      // otherwise fetch direct from Read
      // TODO inefficient
      let mut buf = [0u8];
      if let Ok(()) = self.reader.read_exact(&mut buf) {
        self.count += 1;
        buf[0] as c_char
      } else {
        // RapidJSON seems to interpret this as eof
        0 as c_char
      }
    }
  }

  // position in stream
  #[inline]
  fn Tell(self : &RustStream) -> usize {
    self.count
  }

  // Apparently these are not necessary for read-only rapidjson::Stream
  #[allow(unused_variables)]
  unsafe fn PutBegin(self : &mut RustStream) -> c_char { unimplemented!("PutBegin not necessary for read-only stream") }
  #[allow(unused_variables)]
  fn Put(self : &mut RustStream, one : c_char) { unimplemented!("Put not necessary for read-only stream") }
  #[allow(unused_variables)]
  fn Flush(self : &mut RustStream) { unimplemented!("Flush not necessary for read-only stream") }
  #[allow(unused_variables)]
  unsafe fn PutEnd(self : &mut RustStream, stuff : *mut c_char) -> usize { unimplemented!("PutEnd not necessary for read-only stream") }
}

use std::cell::RefCell;

// convert rapidjson values to JsonEvents and send to a channel.
pub struct RustHandler
{
  tx : RefCell<rtrb::Producer<JsonEvent<String>>>,
}

use crate::parser::JsonEvent;

impl RustHandler {
  pub fn new(tx : rtrb::Producer<JsonEvent<String>>) -> Self
  {
    Self{tx: RefCell::new(tx)}
  }

  pub fn close(self) {
    drop(self.tx)
  }

  // shim to ease the forwarding
  #[inline]
  fn send(&self, jev : JsonEvent<String>) -> bool {
    let mut tx = self.tx.borrow_mut();
    while !tx.is_abandoned() {
      let jev = jev.clone();
      match tx.push(jev) {
        Ok(()) => return true,
        Err(rtrb::PushError::Full(_)) => {
          // ringbuffer is full, so wait for signal from consumer
          std::thread::park()
        }
      }
    }
    return false
  }

  // return value for all of these is true -> continue parsing; false -> halt parsing
  //
  // the _copy parameter appears to be always 'true', which I take to mean "it's not safe to keep a reference to this parameter"
  fn Null(self : &RustHandler) -> bool { self.send(JsonEvent::Null) }
  fn Bool(self : &RustHandler, val : bool) -> bool { self.send(JsonEvent::Boolean(val)) }

  // All the number types.
  //
  // TODO rapidjson has already parsed these strings into numbers, so it's
  // wasteful to convert them back to strings.
  fn Int(self : &RustHandler, val : i32) -> bool { self.send(JsonEvent::Number(format!("{val}"))) }
  fn Uint(self : &RustHandler, val : u64) -> bool { self.send(JsonEvent::Number(format!("{val}"))) }
  fn Int64(self : &RustHandler, val : i64) -> bool { self.send(JsonEvent::Number(format!("{val}"))) }
  fn Uint64(self : &RustHandler, val : i64) -> bool { self.send(JsonEvent::Number(format!("{val}"))) }
  fn Double(self : &RustHandler, val : f64) -> bool { self.send(JsonEvent::Number(format!("{val}"))) }
  fn RawNumber(self : &RustHandler, val : *const c_char, length : usize, _copy : bool) -> bool {
    let val = unsafe { std::slice::from_raw_parts(val as *const u8, length) };
    let val = unsafe { std::str::from_utf8_unchecked(val) };
    self.send(JsonEvent::Number(val.into()))
  }

  fn String(self : &RustHandler, val : *const c_char, length : usize, _copy : bool) -> bool {
    // TODO there must be a cxx.rss builtin for this
    let val = unsafe { std::slice::from_raw_parts(val as *const u8, length) };
    let val = unsafe { std::str::from_utf8_unchecked(val) };
    self.send(JsonEvent::String(val.into()))
  }

  fn StartObject(self : &RustHandler) -> bool { self.send(JsonEvent::StartObject) }
  fn Key(self : &RustHandler, val : *const c_char, length : usize, _copy : bool) -> bool {
    // TODO there must be a cxx.rss builtin for this
    let val = unsafe { std::slice::from_raw_parts(val as *const u8, length) };
    let val = unsafe { std::str::from_utf8_unchecked(val) };
    self.send(JsonEvent::ObjectKey(val.into()))
  }
  fn EndObject(self : &RustHandler, _member_count : usize) -> bool { self.send(JsonEvent::EndObject) }
  fn StartArray(self : &RustHandler) -> bool { self.send(JsonEvent::StartArray) }
  fn EndArray(self : &RustHandler, _element_count : usize) -> bool { self.send(JsonEvent::EndArray) }
}

// can also have
// #[cxx::bridge(namespace = "your_namespace_here")]
// along with relevant namespaces in c++ wrapper
#[cxx::bridge]
pub mod ffi {
    // Shared structures

    // (None)

    // These must be implemented in Rust
    extern "Rust" {
      // implement the api required by rapidjson
      type RustStream;
      type RustHandler;

      // RustStream methods
      fn Peek(self : &mut RustStream) -> c_char;
      fn Take(self : &mut RustStream) -> c_char;
      // position in stream
      fn Tell(self : &RustStream) -> usize;
      unsafe fn PutBegin(self : &mut RustStream) -> c_char;
      fn Put(self : &mut RustStream, one : c_char);
      fn Flush(self : &mut RustStream);
      unsafe fn PutEnd(self : &mut RustStream, stuff : *mut c_char) -> usize;

      // RustHandler methods
      // These are callbacks from c++
      // This implements the same api as the Handler concept in the rapidjson c++
      fn Null(self : &RustHandler) -> bool;
      fn Bool(self : &RustHandler, b : bool) -> bool;
      fn Int(self : &RustHandler, i : i32) -> bool;
      fn Uint(self : &RustHandler, i : u64) -> bool;
      fn Int64(self : &RustHandler, i : i64) -> bool;
      fn Uint64(self : &RustHandler, i : i64) -> bool;
      fn Double(self : &RustHandler, d : f64) -> bool;
      unsafe fn RawNumber(self : &RustHandler, val : *const c_char, length : usize, copy : bool) -> bool;
      unsafe fn String(self : &RustHandler, val : *const c_char, length : usize, copy : bool) -> bool;
      fn StartObject(self : &RustHandler) -> bool;
      unsafe fn Key(self : &RustHandler, val : *const c_char, length : usize, copy : bool) -> bool;
      fn EndObject(self : &RustHandler, member_count : usize) -> bool;
      fn StartArray(self : &RustHandler) -> bool;
      fn EndArray(self : &RustHandler, element_count : usize) -> bool;
    }

    unsafe extern "C++" {
      // NOTE jch is just what cxx-build wants to call it, because of the cargo package name.
      include!("jch/src/wrapper.h");

      // These functions must be implemented in c++
      pub fn parse(handler : &mut RustHandler, istream : &mut RustStream);
      pub fn from_file(filename : String, handler : &mut RustHandler);
    }
}

// This seems to be around optimal
const RING_BUFFER_BOUND : usize = (2usize).pow(12);

/// parse events via our implementation of a rapidjson::Stream.
/// It's quite slow compared to letting rapidjson handle the file reading.
pub fn parse( istream : Box<dyn std::io::BufRead> ) {
  // let src : &[u8] = r#"{"one": "uno", "two": 2, "tre": false}"#.as_bytes();
  // let istream = Box::new(src);
  let (tx, mut rx) = rtrb::RingBuffer::new(RING_BUFFER_BOUND);

  let cons_thr = std::thread::spawn(move || {
    while let Ok(event) = rx.pop() {
      println!("{event:?}");
    }
  });

  let mut reader = RustStream::new(istream);
  let mut handler = RustHandler::new(tx);
  ffi::parse(&mut handler, &mut reader);

  cons_thr.join().unwrap()
}

/// Shim to present a channel as a JsonEvents pull source.
struct ChannelStreamer(rtrb::Consumer<JsonEvent<String>>, std::thread::Thread);

use crate::parser::JsonEvents;

impl<'l> JsonEvents<'_,String> for ChannelStreamer {
  #[inline]
  fn next_event<'a>(&'a mut self) -> std::result::Result<JsonEvent<std::string::String>, Box<(dyn std::error::Error + 'static)>> {
    while !self.0.is_abandoned() {
      match self.0.pop() {
        Ok(jev) => return Ok(jev),
        Err(rtrb::PopError::Empty) => {
          // tell the producer to carry on
          self.1.unpark();
          continue
        },
      }
    }
    Ok(JsonEvent::Eof)
  }
}

/// This constructs a rapidjson parser from the filename, thereby maximising read performance,
/// and then sends event from rapidjson's push parser to a channel, which feeds our pull-oriented schema calculator.
///
/// In fact using this setup, the receive/schema thread is slower than the send/parser thread
/// which only operates at about 65% capacity.
pub fn schema_from_file( filename : &str ) {
  let (tx, rx) = rtrb::RingBuffer::new(RING_BUFFER_BOUND);

  let mut streamer = ChannelStreamer(rx, std::thread::current());
  let cons_thr = std::thread::Builder::new()
    .name("jch rapid recv".into())
    .spawn( move || crate::schema::schema(&mut std::io::stdout(), &mut streamer) )
    // it's no-go if the receive thread can't be created, so just die.
    .expect("cannot create recv thread");

  let mut handler = RustHandler::new(tx);
  ffi::from_file(filename.to_string(), &mut handler );

  // Shut down channel. Kak api because if you forget to call this, the thread just blocks.
  handler.close();
  cons_thr.join().unwrap()
}
