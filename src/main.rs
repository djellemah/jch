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


pub struct JsonEvents {
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
  // TODO move error handling into next_buf
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
      Step::Index(v) => write!(f, "{v}"),
    }
  }
}

impl std::fmt::Octal for Step {
  fn fmt(&self, f : &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    match &self {
      Step::Key(v) => write!(f, "\"{v}\""),
      Step::Index(v) => write!(f, "{v}"),
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
type Parents = rpds::Vector<Step>;
// type Parents = rpds::List<Step>;
type JsonPath = Parents;

mod jsonpath {
  use super::JsonPath;

  #[derive(Debug)]
  pub struct SendPath(JsonPath);

  impl From<&JsonPath> for SendPath {
    fn from(jsonpath : &JsonPath) -> Self {
      Self(jsonpath.clone())
    }
  }

  // This produces jq-equivalent notation
  impl std::fmt::Octal for SendPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
      let string_parts = self.0.iter().map(|step| format!("{step:o}")).collect::<Vec<String>>();
      let repr = string_parts.join(",");
      write!(f,"[{repr}]")
    }
  }

  impl std::fmt::Display for SendPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      let string_parts = self.0.iter().map(ToString::to_string).collect::<Vec<String>>();
      let repr = string_parts.join("/");

      write!(f,"{repr}")
    }
  }
}

mod sendpath {
  use super::JsonPath;
  use super::Step;

  struct SendPath(Vec<Step>);
  impl From<&JsonPath> for SendPath {
    fn from(path_list : &JsonPath) -> Self {
      let steps = path_list.iter().map(std::clone::Clone::clone).collect::<Vec<Step>>();
      // steps.reverse(); for list
      Self(steps)
    }
  }

  impl std::fmt::Display for SendPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      let string_parts = self.0.iter().map(std::string::ToString::to_string).collect::<Vec<String>>();
      let repr = string_parts.join("/");

      write!(f,"{repr}")
    }
  }
}

type SendPath = jsonpath::SendPath;

#[derive(Debug)]
enum Event {
  Path(u64,SendPath),
  Value(SendPath,serde_json::Value),
  Finished,
}

trait Sender<T> {
  type SendError;
  fn send(&self, t: T) -> Result<(), Self::SendError>;
}

// This is a lot of machinery just to call a function :-\
mod fn_snd {
  pub struct FnSnd<T>(fn(T) -> ());

  // This is identical to std::sync::mpsc::SendError
  #[derive(Debug)]
  pub struct SendError<T>(pub T);

  impl<T> super::Sender<T> for FnSnd<T> {
    type SendError = SendError<T>;

    fn send(&self, t: T) -> Result<(), SendError<T>> {
      Ok(self.0(t))
    }
  }

  use super::JsonEvents;
  use super::JsonPath;
  use super::Event;

  #[allow(dead_code)]
  pub fn values(jev : &mut JsonEvents) {
    // return true if handler should be called for this path
    let match_path = |_json_path : &JsonPath| {
      true
      // match json_path.first() {
      //   Some(super::Step::Key(v)) => &v[..] == "annotations",
      //   Some(super::Step::Index(_n)) => false,
      //   None => false,
      // }
    };

    // call handler with specified paths
    let handler = FnSnd(|t : Event| {
      match t {
        // Event::Path(_depth,_path) => (),
        Event::Path(depth,path) => println!("{depth},{path}"),
        Event::Value(p,v) => println!("[{p:#o},{v}]"),
        Event::Finished => (),
      }
    });

    use super::Handler;
    let visitor = super::Valuer(match_path);
    match visitor.find_path::<FnSnd<Event>>(jev, JsonPath::new(), 0, &handler ) {
      Ok(()) => (),
      Err(err) => { eprintln!("ending event reading {err:?}") },
    }
  }

  pub fn paths(jev : &mut JsonEvents) {
    // call handler with specified paths
    let handler = FnSnd(|t : Event| {
      match t {
        Event::Path(depth,path) => println!("{depth},{path}"),
        Event::Value(p,v) => println!("{p} => {v}"),
        Event::Finished => (),
      }
    });

    use super::Handler;
    let visitor = super::Plain;
    match visitor.find_path::<FnSnd<Event>>(jev, JsonPath::new(), 0, &handler ) {
      Ok(()) => (),
      Err(err) => { eprintln!("ending event reading {err:?}") },
    }
  }
}

// for sending the same Path representation over the channel as the one that's constructed
#[allow(unused_macros)]
macro_rules! package_same {
  ($tx:ident,$depth:ident,&$parents:expr) => {
    $tx.send( Some(($depth,$parents.clone())) )
  };
  ($tx:ident,$depth:ident,$parents:expr) => {
    $tx.send(Some(($depth,$parents)))
  };
}

// send a different Path representation over the channel.
macro_rules! package {
  // see previous to distinguish where clone() is needed
  ($tx:ident,0,&$parents:expr) => {
    $tx.send( Event::Path(0, SendPath::from($parents)) )
  };
  ($tx:ident,0,$parents:expr) => {
    $tx.send( Event::Path(0, SendPath::from($parents)) )
  };
  ($tx:ident,$depth:ident,&$parents:expr) => {
    $tx.send( Event::Path(0, SendPath::from($parents)) )
  };
  ($tx:ident,$depth:ident,$parents:expr) => {
    $tx.send( Event::Path(0, SendPath::from($parents)) )
  };
}

// This really just becomes a place to hang match_path and maybe_send_value without threading
// those functions through the JsonEvent handlers.
trait Handler {
  fn match_path(&self, path : &JsonPath) -> bool;

  // default implementation that does nothing and returns OK
  #[allow(unused_variables)]
  fn maybe_send_value<Snd : Sender<Event>>(&self, path : &JsonPath, ev : &json_event_parser::JsonEvent, tx : &Snd) -> Result<(),Snd::SendError>;

  fn count_array<Snd : Sender<Event>>(&self, jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &Snd ) -> Result<(),Snd::SendError> {
    let mut index = 0;
    let mut buf : Vec<u8> = vec![];
    while let Some(ev) = jev.next_buf(&mut buf) {
      let loop_parents = parents.push_back(index.into());
      use json_event_parser::JsonEvent::*;
      let res = match ev {
        // ok we have a leaf, so display the path
        String(_) | Number(_)  | Boolean(_) | Null => self.maybe_send_value(&parents, &ev, tx),

        StartArray => self.count_array(jev, loop_parents, depth+1, tx),
        EndArray => return Ok(()), // do not send path, this is +1 past the end of the array

        // ObjectKey(key) => find_path(jev, loop_parents.push_front(key.into()), depth+1, tx),
        StartObject => self.handle_object(jev, loop_parents, depth+1, tx),
        ObjectKey(_) => panic!("should never receive ObjectKey {parents}"),
        EndObject => panic!("should never receive EndObject {parents}"),

        Eof => tx.send(Event::Finished),
      };
      match res {
          Ok(()) => (),
          err => return err,
      }
      index += 1;
    }
    Ok(())
  }

  fn handle_object<Snd : Sender<Event>>(&self, jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &Snd ) -> Result<(),Snd::SendError> {
    let mut buf : Vec<u8> = vec![];
    while let Some(ev) = jev.next_buf(&mut buf) {
      use json_event_parser::JsonEvent::*;
      let res = match ev {
        // ok we have a leaf, so display the path
        String(_) | Number(_)  | Boolean(_) | Null => self.maybe_send_value(&parents, &ev, tx),

        StartArray => self.count_array(jev, parents.clone(), depth+1, tx),
        EndArray => panic!("should never receive EndArray {parents}"),

        StartObject => self.find_path(jev, parents.clone(), depth+1, tx),
        ObjectKey(key) => self.find_path(jev, parents.push_back(key.into()), depth+1, tx),
        EndObject => return Ok(()),

        // fin
        Eof => tx.send(Event::Finished),
      };
      match res {
          Ok(()) => (),
          err => return err,
      }
    }
    Ok(())
  }

  fn find_path<Snd : Sender<Event>>(&self, jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &Snd ) -> Result<(),Snd::SendError> {
    let mut buf : Vec<u8> = vec![];
    // json has exactly one top-level object
    if let Some(ev) = jev.next_buf(&mut buf) {
      use json_event_parser::JsonEvent::*;
      match ev {
        // ok we have a leaf, so display the path
        String(_) | Number(_)  | Boolean(_) | Null => self.maybe_send_value(&parents, &ev, tx),

        StartArray => self.count_array(jev, parents, depth+1, tx),
        EndArray => panic!("should never receive EndArray {parents}"),

        StartObject => self.handle_object(jev, parents, depth+1, tx),
        ObjectKey(_) => panic!("should never receive ObjectKey {parents}"),
        EndObject => panic!("should never receive EndObject {parents}"),

        // fin
        Eof => tx.send(Event::Finished),
      }
    } else {
      tx.send(Event::Finished)
    }
  }
}

struct Plain;

impl Handler for Plain {
  // default implementation that does nothing and returns OK
  #[allow(unused_variables)]
  fn maybe_send_value<Snd : Sender<Event>>(&self, path : &JsonPath, ev : &json_event_parser::JsonEvent, tx : &Snd) -> Result<(),Snd::SendError> {
    println!("{path}");
    Ok(())
  }

  fn match_path(&self, _path : &JsonPath) -> bool {
    println!("{_path}");
    // ensure all paths are sent
    // if this was true, maybe_send_values would be called with the value as well.
    false
  }
}

struct Valuer(fn(&JsonPath) -> bool);

impl Handler for Valuer {
  fn match_path(&self, path: &JsonPath) -> bool {
    self.0(path)
  }

  fn maybe_send_value<Snd : Sender<Event>>(&self, path : &JsonPath, &ev : &json_event_parser::JsonEvent, tx : &Snd)
  -> Result<(),Snd::SendError> {
    use json_event_parser::JsonEvent::*;
    match ev {
      String(v) => if self.match_path(&path) {
        let value = serde_json::Value::String(v.into());
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(Event::Value(SendPath::from(path),value))
      } else {
        // just send the path
        package!(tx,0,path)
      }
      Number(v) => if self.match_path(&path) {
        let value : serde_json::Number = match serde_json::from_str(v) {
            Ok(n) => n,
            Err(msg) => panic!("{v} appears to be not-a-number {msg}"),
        };
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(Event::Value(SendPath::from(path), serde_json::Value::Number(value)))
      } else {
        // just send the path
        package!(tx,0,path)
      }
      Boolean(v) => if self.match_path(&path) {
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(Event::Value(SendPath::from(path), serde_json::Value::Bool(v)))
      } else {
        // just send the path
        package!(tx,0,path)
      },
      Null => if self.match_path(&path) {
        // let path = path.iter().map(|s| s.clone()).collect::<Vec<Step>>();
        tx.send(Event::Value(SendPath::from(path), serde_json::Value::Null))
      } else {
        // just send the path
        package!(tx,0,path)
      },
      _ => todo!(),
    }
  }
}

fn main() {
  let istream = make_readable();
  let mut jev = JsonEvents::new(istream);
  // ch_snd::channels(&mut jev);
  fn_snd::values(&mut jev);
}
