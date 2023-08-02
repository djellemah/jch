#![feature(generators, generator_trait)]
// https://serde.rs/stream-array.html
// seems to be a bit clunky

// actually designed for streaming, but seems a bit incomplete
// https://github.com/Marcono1234/struson

//
// https://github.com/alexmaco/json_stream


#[allow(dead_code)]
fn keys_only(pit : &mut json_stream::parse::ParseObject) {
  while let Some(parse_result) = pit.next() {
    match parse_result {
      Ok(mut kv) => {
        match kv.key().read_owned() {
          Ok(key) => {
            println!("key: {key:?}");
            // drop(kv); should be called automatically anyway
          }
          Err(err) => eprintln!("read key failed {err:?}"),
        }
      }
      Err(v) => eprintln!("read kv failed {v:?}"),
    };
    println!("read next key")
  }
}


#[allow(dead_code)]
fn old_main() {
  let istream = make_readable();
  let mut top_stream = json_stream::parse::Parser::new(istream);
  while let Some(pobject) = top_stream.next() {
    match pobject {
      Ok(mut parse_object) => {
        use json_stream::parse::Json::*;
        match &mut parse_object {
          Null => println!("Null"),
          Bool(_) => println!("Bool"),
          Number(_) => println!("Number"),
          String(_) => println!("String"),
          Array(_parse_array) => {println!("Array")},
          Object(parse_object) => {
            // println!("Object {:?}", parse_object);
            keys_only(parse_object);
            drop(parse_object)
          }
        }
      }
      Err(err) => eprintln!("{err:?}"),
    }
  }
}

fn make_readable() -> Box<dyn std::io::BufRead> {
  let args = std::env::args().collect::<Vec<_>>();
  // use std::io::Read;
  match &args[..] {
    [_] => Box::new(std::io::stdin().lock()),
    [_, arg_fn] => {
      let file = std::fs::File::open(arg_fn).expect("cannot open file {arg_fn}");
      Box::new(std::io::BufReader::new(file))
    }
    _ => panic!("too many args")
  }
}



fn main() {
  let istream = make_readable();
  // json_event_parser::{JsonReader, JsonEvent};
  // use std::io::Cursor;
  // use std::io::BufRead;


  // let mut reader = json_event_parser::JsonReader::from_reader(Cursor::new(istream));
  let mut reader = json_event_parser::JsonReader::from_reader(istream);
  let mut buf : Vec<u8> = vec![];
  loop {
    match reader.read_event(&mut buf).unwrap() {
      json_event_parser::JsonEvent::Eof => break,
      event => println!("{event:?}"),
    }
  }
}
