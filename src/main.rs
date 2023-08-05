// type StrCon<T> = std::rc::Rc<T>;
type StrCon<T> = Box<T>;

// trait PosReader : std::io::BufRead + std::io::Seek {}
// trait PosReader : std::io::BufRead {}

fn make_readable() -> StrCon<dyn std::io::BufRead> {
  let args = std::env::args().collect::<Vec<_>>();
  // use std::io::Read;
  match &args[..] {
    [_] => StrCon::new(std::io::stdin().lock()),
    [_, arg_fn] => {
      let file = std::fs::File::open(arg_fn).expect("cannot open file {arg_fn}");
      StrCon::new(std::io::BufReader::new(file))
    }
    _ => panic!("too many args")
  }
}


struct JsonEvents {
  // reader : json_event_parser::JsonReader<Box<dyn std::io::BufRead>>,
  reader : json_event_parser::JsonReader<Box<countio::Counter<Box<dyn std::io::BufRead>>>>,
  // counter : &'a Box<countio::Counter<Box<dyn std::io::BufRead>>>,
  buf : Vec<u8>,
}

impl JsonEvents {
  fn new(istream : StrCon<dyn std::io::BufRead>) -> Self {
    let counter = Box::new(countio::Counter::new(istream));
    // let rcounter = &counter;
    let reader = json_event_parser::JsonReader::from_reader(counter);
    let buf : Vec<u8> = vec![];
    // Self{reader, counter: rcounter, buf}
    Self{reader, buf}
  }

  // it's a severe PITA to specify this as an implementation of Iterator
  fn next(&mut self) -> Option<json_event_parser::JsonEvent> {
    match self.reader.read_event(&mut self.buf) {
      Ok(ev) => match ev {
        json_event_parser::JsonEvent::Eof => None,
        event => Some(event),
      }
      Err(err) => {
        let counter = &self.reader.reader as &countio::Counter<Box<dyn std::io::BufRead>>;
        let pos = counter.reader_bytes();
        // let mut buf : Vec<u8> = Vec::with_capacity(25);
        // try to generate some context
        let mut buf = [0; 80];
        use std::io::Read;
        let more = match self.reader.reader.read(&mut buf) {
          Ok(n) => String::from_utf8_lossy(&buf[0..n]).to_string(),
          Err(err) => format!("{err:?}"),
        };

        eprintln!("pos {pos} {err} followed by {more}");

        // Some(json_event_parser::JsonEvent::Null)
        None
      }
    }
  }

  #[allow(dead_code)]
  fn next_buf<'a, 'b>(&'a mut self, buf : &'b mut Vec<u8>) -> Option<json_event_parser::JsonEvent<'b>> {
    match self.reader.read_event(buf) {
      Ok(json_event_parser::JsonEvent::Eof) => None,
      Ok(event) => Some(event),
      Err(_) => None,
    }
  }
}

// json_event_parser::JsonEvent::String(_) => todo!(),
// json_event_parser::JsonEvent::Number(_) => todo!(),
// json_event_parser::JsonEvent::Boolean(_) => todo!(),
// json_event_parser::JsonEvent::Null => todo!(),
// json_event_parser::JsonEvent::StartArray => todo!(),
// json_event_parser::JsonEvent::EndArray => todo!(),
// json_event_parser::JsonEvent::StartObject => todo!(),
// json_event_parser::JsonEvent::EndObject => todo!(),
// json_event_parser::JsonEvent::ObjectKey(_) => todo!(),
// json_event_parser::JsonEvent::Eof => todo!(),

use json_event_parser::JsonEvent;

#[allow(dead_code)]
fn show_all(jev : &mut JsonEvents) {
  while let Some(ev) = jev.next() {
    println!("{ev:?}");
  }
}

#[allow(dead_code)]
fn is_object(jev : &mut JsonEvents) -> bool {
  while let Some(ev) = jev.next() {
    match ev {
      JsonEvent::StartObject => return true,
      _ => return false,
    }
  }
  false
}

// This is basically xpath or jql in disguise
#[allow(dead_code)]
fn ignore(jev : &mut JsonEvents) {
  while let Some(ev) = jev.next() {
    match ev {
      JsonEvent::StartObject => ignore(jev),
      JsonEvent::EndObject => return,
      _ => (),
    }
  }
}

#[derive(Debug,Clone)]
enum Step {
  Key(String),
  Index(u64),
}

impl Step {
  #[allow(dead_code)]
  fn plusone(&self) -> Self {
    match &self {
      Step::Key(v) => panic!("{v} is not an integer"),
      Step::Index(v) => Step::Index(v+1),
    }
  }
}

impl std::fmt::Display for Step {
  fn fmt(&self, f : &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    match &self {
      Step::Key(v) => write!(f, "{v}"),
      Step::Index(v) => write!(f, "[{v}]"),
    }
  }
}


impl From<&str> for Step {
  fn from(s: &str) -> Self { Self::Key(s.to_string()) }
}

impl From<u64> for Step {
  fn from(s: u64) -> Self { Self::Index(s) }
}

// https://docs.rs/rpds/latest/rpds/list/struct.List.html
// type Parents = rpds::List<Step, archery::ArcK>;
type Parents = rpds::List<Step>;
// type Parents = rpds::List<Step>;
type JsonPath = Parents;

fn make_indent(parents : &Parents) -> String {
  let mut indent = String::new();
  for _ in parents { indent.push(' ') };
  indent
}

fn collect_keys(jev : &mut JsonEvents, parents : &Parents) {
  let mut map : std::collections::BTreeMap<String, serde_json::Value> = std::collections::BTreeMap::new();

  // eurgh. This is a rather unpleasant pattern
  let mut buf : Vec<u8> = vec![];
  if let Some(ev) = jev.next_buf(&mut buf) {
    match ev {
      JsonEvent::ObjectKey(key) => {
        map.insert(key.to_string(), collect_value(jev, key, &parents));
        collect_keys(jev, parents);
      }
      JsonEvent::Null => todo!(),
      JsonEvent::StartArray => todo!(),
      JsonEvent::EndArray => todo!(),
      JsonEvent::StartObject => todo!(),
      JsonEvent::EndObject => todo!(),
      other => panic!("unhandled {other:?}"),
    }
  }
}

fn collect_value(jev : &mut JsonEvents, key : &str, parents : &Parents) -> serde_json::Value {
  let indent = make_indent(parents);
  if let Some(ev) = jev.next() {
    match ev {
      JsonEvent::String(val) => {
        println!("{indent}{key}: {val}");
        serde_json::Value::String(val.to_string())
      }
      JsonEvent::Number(val) => {
        println!("{indent}{key}");
        serde_json::Value::String(val.to_string())
        // serde_json::Value::Number(val.parse::<i64>().unwrap_or(serde_json::Value::Null))
      }
      _ => serde_json::Value::Null,
    }
  } else {
    serde_json::Value::Null
  }
}

#[allow(dead_code)]
fn display_keys(jev : &mut JsonEvents, parents : &Parents) {
  let mut indent = String::new();
  for _ in parents { indent.push(' ') };
  let mut map : std::collections::BTreeMap<String, Option<serde_json::Value>> = std::collections::BTreeMap::new();

  let mut buf : Vec<u8> = vec![];
  while let Some(ev) = jev.next_buf(&mut buf) {
    match ev {
      JsonEvent::StartObject => {
        println!("---");
        // display_keys(jev, &parents)
        collect_keys(jev, &parents)
      }
      JsonEvent::EndObject => return,
      JsonEvent::ObjectKey(key) => {
        println!("{indent}{key}");
        map.insert(key.to_string(), None);
      }
      JsonEvent::Eof => panic!("unexpected eof"),
      _ => (),
    }
  }
}

fn path_to_string( path : & Parents ) -> String {
  let mut parent_vec = path.iter().map(|e| e.to_string()).collect::<Vec<String>>();
  parent_vec.reverse();
  parent_vec.join("/")
}

// type Snd = std::sync::mpsc::Sender<Option<JsonPath>>;
type SendPath = Vec<Step>;
type Event = Option<(u64,SendPath)>;
type Snd = std::sync::mpsc::SyncSender<Event>;
type SndResult = Result<(), std::sync::mpsc::SendError<Event>>;

// fn package(path : &JsonPath) -> Vec<Step> {
//   path.iter().map(|step| step.clone()).collect::<Vec<Step>>()
// }
// macro_rules! package {
//   ($tx:ident,$depth:ident,&$parents:expr) => {
//     $tx.send( Some(($depth,$parents.clone())) )
//   };
//   ($tx:ident,$depth:ident,$parents:expr) => {
//     $tx.send(Some(($depth,$parents)))
//   };
// }

macro_rules! package {
  ($tx:ident,$depth:ident,&$parents:expr) => {
    $tx.send( Some(( $depth, $parents.iter().map(|s| s.clone()).collect::<Vec<Step>>() )) )
  };
  ($tx:ident,$depth:ident,$parents:expr) => {
    $tx.send(Some(( $depth, $parents.iter().map(|s| s.clone()).collect::<Vec<Step>>() )))
  };
}

fn count_array(jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &Snd ) -> SndResult {
  let mut index = 0;
  let mut buf : Vec<u8> = vec![];
  while let Some(ev) = jev.next_buf(&mut buf) {
    let loop_parents = parents.push_front(index.into());
    use json_event_parser::JsonEvent::*;
    let res = match ev {
      String(_) => package!(tx,depth,loop_parents),
      Number(_) => package!(tx,depth,loop_parents),
      Boolean(_) => package!(tx,depth,loop_parents),
      Null => package!(tx,depth,loop_parents),

      StartArray => count_array(jev, loop_parents, depth+1, tx),
      EndArray => return package!(tx,depth,loop_parents),
      // ObjectKey(key) => find_path(jev, loop_parents.push_front(key.into()), depth+1, tx),
      StartObject => handle_object(jev, loop_parents, depth+1, tx),
      ObjectKey(_) => panic!("should never receive ObjectKey in count_array {}", path_to_string(&parents)),
      EndObject => panic!("should never receive EndObject in count_array {}", path_to_string(&parents)),
      Eof => tx.send(None),
    };
    match res {
        Ok(()) => (),
        err => return err,
    }
    index += 1;
  }
  Ok(())
}

fn handle_object(jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &Snd ) -> SndResult {
  let mut buf : Vec<u8> = vec![];
  use json_event_parser::JsonEvent::*;
  while let Some(ev) = jev.next_buf(&mut buf) {
    let res = match ev {
      // ok we have a leaf, so display the path
      String(_) => package!(tx,depth,&parents),
      Number(_) => package!(tx,depth,&parents),
      Boolean(_) => package!(tx,depth,&parents),
      Null => package!(tx,depth,&parents),

      StartArray => count_array(jev, parents.clone(), depth+1, tx),
      EndArray => panic!("should never receive EndArray in handle_object {}", path_to_string(&parents)),
      ObjectKey(key) => find_path(jev, parents.push_front(key.into()), depth+1, tx),
      StartObject => find_path(jev, parents.clone(), depth+1, tx),
      EndObject => return Ok(()),
      // fin
      Eof => tx.send(None),
    };
    match res {
        Ok(()) => (),
        err => return err,
    }
  }
  Ok(())
}

fn find_path(jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &Snd ) -> SndResult {
  let mut buf : Vec<u8> = vec![];
  use json_event_parser::JsonEvent::*;
  if let Some(ev) = jev.next_buf(&mut buf) {
    match ev {
      // ok we have a leaf, so display the path
      String(_) => package!(tx,depth,parents),
      Number(_) => package!(tx,depth,parents),
      Boolean(_) => package!(tx,depth,parents),
      Null => package!(tx,depth,parents),

      StartArray => count_array(jev, parents, depth+1, tx),
      EndArray => panic!("should never receive EndArray in find_path {}", path_to_string(&parents)),
      ObjectKey(_) => panic!("should never receive ObjectKey in find_path {}", path_to_string(&parents)),
      StartObject => handle_object(jev, parents, depth+1, tx),
      EndObject => panic!("should never receive EndObject in find_path {}", path_to_string(&parents)),
      // fin
      Eof => tx.send(None),
    }
  } else {
    tx.send(None)
  }
}

fn append_step(steps : &Vec<Step>, last : Step) -> Vec<Step> {
  let mut more_steps = steps.clone();
  more_steps.push(last);
  more_steps
}

fn channels(jev : &mut JsonEvents) {
  let (tx, rx) = std::sync::mpsc::sync_channel::<Event>(4096);

  // consumer thread
  std::thread::spawn(move || {
    loop {
      match rx.recv() {
        Ok(Some((depth,path))) => {
          println!("{depth}:{}", path.iter().map(|s| s.to_string()).collect::<Vec<String>>().join("/"))
        },
        Ok(None) => break,
        Err(err) => { eprintln!("ending consumer: {err}"); break },
      }
    }
  });

  // producer loop pass the event source (jev) to the
  // parsers, then
  loop {
    match find_path(jev, JsonPath::new(), 0, &tx) {
      Ok(()) => (),
      Err(err) => { eprintln!("ending producer {err}"); break },
    }
  }
}

fn main() {
  let istream = make_readable();
  let mut jev = JsonEvents::new(istream);
  channels(&mut jev);
}
