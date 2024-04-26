// #![allow(non_upper_case_globals)]
// #![allow(non_camel_case_types)]
#![allow(non_snake_case)]
// include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// https://cxx.rs/

use std::ffi::c_char;

struct RustStream {
  reader : Box<dyn std::io::BufRead>,
  peeked : Option<c_char>,
  count : usize,
}

#[allow(unused_variables)]
impl RustStream {
  fn Peek(self : &mut RustStream) -> c_char {
    if let Some(peeked) = self.peeked {
      peeked
    } else {
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

  fn Take(self : &mut RustStream) -> c_char {
    if let Some(peeked) = self.peeked {
      let rv = peeked as c_char;
      self.peeked = None;
      rv
    } else {
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
  fn Tell(self : &RustStream) -> usize {
    self.count
  }

  unsafe fn PutBegin(self : &mut RustStream) -> c_char {
    todo!("implement PutBegin")
  }

  fn Put(self : &mut RustStream, one : c_char) { todo!("Put") }
  fn Flush(self : &mut RustStream) { todo!("Flush") }
  unsafe fn PutEnd(self : &mut RustStream, stuff : *mut c_char) -> usize { todo!("PutEnd") }
}

struct RustHandler;

impl RustHandler {
  fn Null(self : &RustHandler) -> bool { println!("null"); true }
  fn Bool(self : &RustHandler, val : bool) -> bool { println!("{val:?}"); true }
  fn Int(self : &RustHandler, val : i32) -> bool { println!("{val:?}"); true }
  fn Uint(self : &RustHandler, val : u64) -> bool { println!("{val:?}"); true }
  fn Int64(self : &RustHandler, val : i64) -> bool { println!("{val:?}"); true }
  fn Uint64(self : &RustHandler, val : i64) -> bool { println!("{val:?}"); true }
  fn Double(self : &RustHandler, val : f64) -> bool { println!("{val:?}"); true }
  fn RawNumber(self : &RustHandler, val : *const c_char, length : usize, copy : bool) -> bool { println!("{length}:{copy}:{val:?}"); true }
  fn String(self : &RustHandler, val : *const c_char, length : usize, copy : bool) -> bool { println!("{length}:{copy}:{val:?}"); true }
  fn StartObject(self : &RustHandler) -> bool { println!("start obj"); true }
  fn Key(self : &RustHandler, val : *const c_char, length : usize, copy : bool) -> bool { println!("{length}:{copy}:{val:?}"); true }
  fn EndObject(self : &RustHandler, member_count : usize) -> bool { println!("end obj {member_count}"); true }
  fn StartArray(self : &RustHandler) -> bool { println!("start ary"); true }
  fn EndArray(self : &RustHandler, element_count : usize) -> bool { println!("end ary {element_count}"); true }
}

#[cxx::bridge]
mod ffi {
  // Shared structures

    // These must be implemented in Rust
    extern "Rust" {
      type RustStream;
      type RustHandler;

      fn Peek(self : &mut RustStream) -> c_char;
      fn Take(self : &mut RustStream) -> c_char;
      // position in stream
      fn Tell(self : &RustStream) -> usize;
      unsafe fn PutBegin(self : &mut RustStream) -> c_char;
      fn Put(self : &mut RustStream, one : c_char);
      fn Flush(self : &mut RustStream);
      unsafe fn PutEnd(self : &mut RustStream, stuff : *mut c_char) -> usize;

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

    #[allow(dead_code)]
    unsafe extern "C++" {
      include!("jch/wrapper.hpp");

      // These functions must be implemented in c++
      // return value is just so the compiler doesn't complain about a void member
      // dunno what that's about.
      fn parse(handler : &mut RustHandler, istream : &mut RustStream);
    }
}

#[allow(unused_variables,unused_mut)]
fn main() {
  let src : &[u8] = r#"{"one": "uno", "two": 2, "tre": false}"#.as_bytes();
  let readable = Box::new(src);
  // {
    let mut reader = RustStream{reader: readable, peeked: None, count: 0};
    let mut handler = RustHandler;
    ffi::parse(&mut handler, &mut reader);
  // }
}
