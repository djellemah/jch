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

pub struct RustHandler;

impl RustHandler {
  // return value for all of these is true -> continue parsing; false -> halt parsing
  fn Null(self : &RustHandler) -> bool { println!("null"); true }
  fn Bool(self : &RustHandler, val : bool) -> bool { println!("bool {val}"); true }
  fn Int(self : &RustHandler, val : i32) -> bool { println!("int {val}"); true }
  fn Uint(self : &RustHandler, val : u64) -> bool { println!("uint {val}"); true }
  fn Int64(self : &RustHandler, val : i64) -> bool { println!("int64 {val}"); true }
  fn Uint64(self : &RustHandler, val : i64) -> bool { println!("uint64 {val}"); true }
  fn Double(self : &RustHandler, val : f64) -> bool { println!("double {val}"); true }
  fn RawNumber(self : &RustHandler, val : *const c_char, length : usize, copy : bool) -> bool { println!("number {length}:{copy}:{val:?}"); true }
  fn String(self : &RustHandler, val : *const c_char, length : usize, copy : bool) -> bool {
    // TODO there must be a cxx.rss builtin for this
    let val = unsafe { std::slice::from_raw_parts(val as *const u8, length) };
    let val = unsafe { std::str::from_utf8_unchecked(val) };
    println!("string {length}:{copy}:{val}", );
    true
  }
  fn StartObject(self : &RustHandler) -> bool { println!("start obj"); true }
  fn Key(self : &RustHandler, val : *const c_char, length : usize, copy : bool) -> bool {
    // TODO there must be a cxx.rss builtin for this
    let val = unsafe { std::slice::from_raw_parts(val as *const u8, length) };
    let val = unsafe { std::str::from_utf8_unchecked(val) };
    println!("key {length}:{copy}:{val}", );
    true
  }
  fn EndObject(self : &RustHandler, member_count : usize) -> bool { println!("end obj {member_count}"); true }
  fn StartArray(self : &RustHandler) -> bool { println!("start ary"); true }
  fn EndArray(self : &RustHandler, element_count : usize) -> bool { println!("end ary {element_count}"); true }
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
    }
}

pub fn ping( istream : Box<dyn std::io::BufRead> ) {
  // let src : &[u8] = r#"{"one": "uno", "two": 2, "tre": false}"#.as_bytes();
  // let istream = Box::new(src);
  let mut reader = RustStream::new(istream);
  let mut handler = RustHandler;
  ffi::parse(&mut handler, &mut reader);
}
