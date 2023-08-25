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
  _buf : Vec<u8>,
}

impl JsonEvents {
  fn new(istream : StrCon<dyn std::io::BufRead>) -> Self {
    let counter = Box::new(countio::Counter::new(istream));
    // let rcounter = &counter;
    let reader = json_event_parser::JsonReader::from_reader(counter);
    let buf : Vec<u8> = vec![];
    // Self{reader, counter: rcounter, buf}
    Self{reader, _buf: buf}
  }

  // it's a severe PITA to specify this as an implementation of Iterator
  // TODO move error handling into next_buf
  #[allow(dead_code)]
  fn next(&mut self) -> Option<json_event_parser::JsonEvent> {
    match self.reader.read_event(&mut self._buf) {
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
pub enum Step {
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
enum Event<V> {
  Path(u64,SendPath),
  Value(SendPath,V),
  Finished,
}

trait Sender<T> {
  type SendError;
  fn send(&self, t: T) -> Result<(), Self::SendError>;
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
  type V;

  fn match_path(&self, path : &JsonPath) -> bool;

  // default implementation that does nothing and returns OK
  #[allow(unused_variables)]
  fn maybe_send_value<Snd : Sender<Event<Self::V>>>(&self, path : &JsonPath, ev : &json_event_parser::JsonEvent, tx : &Snd) -> Result<(),Snd::SendError>;

  fn array<Snd : Sender<Event<Self::V>>>(&self, jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &Snd ) -> Result<(),Snd::SendError> {
    let mut index = 0;
    let mut buf : Vec<u8> = vec![];
    while let Some(ev) = jev.next_buf(&mut buf) {
      let loop_parents = parents.push_back(index.into());
      use json_event_parser::JsonEvent::*;
      let res = match ev {
        // ok we have a leaf, so match path then send value
        String(_) | Number(_)  | Boolean(_) | Null => self.maybe_send_value(&parents, &ev, tx),

        StartArray => self.array(jev, loop_parents, depth+1, tx),
        EndArray => return Ok(()), // do not send path, this is +1 past the end of the array

        // ObjectKey(key) => find_path(jev, loop_parents.push_front(key.into()), depth+1, tx),
        StartObject => self.object(jev, loop_parents, depth+1, tx),
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

  fn object<Snd : Sender<Event<Self::V>>>(&self, jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &Snd ) -> Result<(),Snd::SendError> {
    let mut buf : Vec<u8> = vec![];
    while let Some(ev) = jev.next_buf(&mut buf) {
      use json_event_parser::JsonEvent::*;
      let res = match ev {
        // ok we have a leaf, so display the path
        String(_) | Number(_)  | Boolean(_) | Null => self.maybe_send_value(&parents, &ev, tx),

        StartArray => self.array(jev, parents.clone(), depth+1, tx),
        EndArray => panic!("should never receive EndArray {parents}"),

        StartObject => self.value(jev, parents.clone(), depth+1, tx),
        ObjectKey(key) => self.value(jev, parents.push_back(key.into()), depth+1, tx),
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

  fn value<Snd : Sender<Event<Self::V>>>(&self, jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &Snd ) -> Result<(),Snd::SendError> {
    let mut buf : Vec<u8> = vec![];
    // json has exactly one top-level object
    if let Some(ev) = jev.next_buf(&mut buf) {
      use json_event_parser::JsonEvent::*;
      match ev {
        // ok we have a leaf, so display the path
        String(_) | Number(_)  | Boolean(_) | Null => self.maybe_send_value(&parents, &ev, tx),

        StartArray => self.array(jev, parents, depth+1, tx),
        EndArray => panic!("should never receive EndArray {parents}"),

        StartObject => self.object(jev, parents, depth+1, tx),
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

impl Handler for Plain
{
  type V = serde_json::Value;

  // default implementation that does nothing and returns OK
  #[allow(unused_variables)]
  fn maybe_send_value<Snd : Sender<Event<Self::V>>>(&self, path : &JsonPath, ev : &json_event_parser::JsonEvent, tx : &Snd) -> Result<(),Snd::SendError> {
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

impl Handler for Valuer
{
  type V = serde_json::Value;

  fn match_path(&self, path: &JsonPath) -> bool {
    self.0(path)
  }

  fn maybe_send_value<Snd : Sender<Event<Self::V>>>(&self, path : &JsonPath, &ev : &json_event_parser::JsonEvent, tx : &Snd)
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

struct MsgPacker(fn(&JsonPath) -> bool);

impl Handler for MsgPacker {
  type V = Vec<u8>;

  fn match_path(&self, path: &JsonPath) -> bool {
    self.0(path)
  }

  fn maybe_send_value<Snd : Sender<Event<Self::V>>>(&self, path : &JsonPath, &ev : &json_event_parser::JsonEvent, tx : &Snd)
  -> Result<(),Snd::SendError> {
    use json_event_parser::JsonEvent::*;
    match ev {
      String(v) => if self.match_path(&path) {
        let mut buf = vec![];
        match rmp::encode::write_str(&mut buf, &v) {
          Ok(()) => tx.send(Event::Value(SendPath::from(path),buf)),
          Err(err) => panic!("msgpack error {err}")
        }
      } else {
        // just send the path
        package!(tx,0,path)
      }
      Number(v) => if self.match_path(&path) {
        let value : serde_json::Number = match serde_json::from_str(v) {
            Ok(n) => n,
            Err(msg) => panic!("{v} appears to be not-a-number {msg}"),
        };

        let mut buf = vec![];
        match rmp::encode::write_f64(&mut buf, value.as_f64().unwrap()) {
          Ok(()) => tx.send(Event::Value(SendPath::from(path), buf)),
          Err(err) => panic!("msgpack error {err}"),
        }
      } else {
        // just send the path
        package!(tx,0,path)
      }
      Boolean(v) => if self.match_path(&path) {
        let mut buf = vec![];
        match rmp::encode::write_bool(&mut buf, v) {
          Ok(()) => tx.send(Event::Value(SendPath::from(path), buf)),
          Err(err) => panic!("msgpack error {err}"),
        }
      } else {
        // just send the path
        package!(tx,0,path)
      },
      Null => if self.match_path(&path) {
        let mut buf = vec![];
        match rmp::encode::write_nil(&mut buf) {
          Ok(()) => tx.send(Event::Value(SendPath::from(path), buf)),
          Err(err) => panic!("msgpack error {err}"),
        }
      } else {
        // just send the path
        package!(tx,0,path)
      },
      _ => todo!(),
    }
  }
}

// This is a lot of machinery just to call a function :-\
mod fn_snd {
  pub struct FnSnd<T>(pub fn(T) -> ());

  // This is identical to std::sync::mpsc::SendError
  #[derive(Debug)]
  pub struct SendError<T>(pub T);

  impl<T> super::Sender<T> for FnSnd<T> {
    type SendError = SendError<T>;

    fn send(&self, t: T) -> Result<(), SendError<T>> {
      Ok(self.0(t))
    }
  }

  // use super::JsonEvents;
  // use super::JsonPath;
  // use super::Event;

  // #[allow(dead_code)]
  // pub fn values<V>(jev : &mut JsonEvents, match_path : fn(&JsonPath) -> bool)
  // where V : std::fmt::Display
  // {
  //   // call handler with specified paths
  //   let handler = FnSnd(|t : Event<V>| {
  //     match t {
  //       Event::Path(_depth,_path) => (),
  //       // Event::Path(depth,path) => println!("path: {depth},{path}"),
  //       Event::Value(p,v) => println!("jq path: [{p:#o},{v}]"),
  //       Event::Finished => (),
  //     }
  //   });

  //   use super::Handler;
  //   let visitor = super::MsgPacker(match_path);
  //   match visitor.value(jev, JsonPath::new(), 0, &handler ) {
  //     Ok(()) => (),
  //     Err(err) => { eprintln!("ending event reading {err:?}") },
  //   }
  // }

  // #[allow(dead_code)]
  // pub fn paths<DV>(jev : &mut JsonEvents)
  // where
  //   DV: std::fmt::Display + std::fmt::Debug, FnSnd<Event<DV>>: crate::Sender<Event<DV>>
  // {
  //   // call handler with specified paths
  //   let handler = FnSnd(|t : Event<DV>| {
  //     match t {
  //       Event::Path(depth,path) => println!("{depth},{path}"),
  //       Event::Value(p,v) => println!("{p} => {v}"),
  //       Event::Finished => (),
  //     }
  //   });

  //   use super::Handler;
  //   let visitor = super::Plain;
  //   match visitor.value::<FnSnd<Event<DV>>>(jev, JsonPath::new(), 0, &handler ) {
  //     Ok(()) => (),
  //     Err(err) => { eprintln!("ending event reading {err:?}") },
  //   }
  // }
}

#[allow(dead_code, unused_mut, unused_variables)]
fn show_jq_paths() {
  let istream = make_readable();
  let mut jev = JsonEvents::new(istream);
  // ch_snd::channels(&mut jev);

  // return true if handler should be called for this path
  let match_lp_ps_path = |json_path : &JsonPath| {
    if json_path.len() < 3 {return false};

    let trefix = json_path
      .iter()
      .take(3)
      .collect::<Vec<&Step>>();

    // This is pretty horrible. Maybe a DSL would be nicer.
    match &trefix[0..3] {
      [&Step::Key(ref v), &Step::Index(n), &Step::Key(ref u) ] => {
        (n == 1 || n == 3) &&
        &v[..] == "learning_paths" &&
        &u[..] == "problem_sequence"
      }
      _ => false
    }
  };
  // fn_snd::values(&mut jev, match_lp_ps_path);

  // let match_images_path = |json_path : &JsonPath| {
  //   // This is pretty horrible. Maybe a DSL would be nicer.
  //   match &json_path[..] {
  //     [&Step::Key(ref v)] => &v[..] == "images",
  //     _ => false
  //   }
  // };
}

fn main() {
  let istream = make_readable();
  let mut jev = JsonEvents::new(istream);

  // receives events from the streaming parser
  let handler = |t : Event<Vec<u8>>| {
    use std::io::Write;
    match t {
      Event::Path(_depth,_path) => (),
      Event::Value(_p,v) =>
      {
        std::io::stdout().write_all(&v).unwrap()
      },
      Event::Finished => (),
    }
  };
  let handler = fn_snd::FnSnd(handler);

  // filters events from the streaming parser
  let match_images_path = |json_path : &JsonPath| {
    // need the &Step ref for nicer matching below
    let json_path = json_path.iter().collect::<Vec<&Step>>();

    // This is pretty horrible. Maybe a DSL would be nicer.
    match &json_path[..] {
      [&Step::Key(ref v), ..] => &v[..] == "images",
      _ => false
    }
  };

  let visitor = MsgPacker(match_images_path);
  match visitor.value(&mut jev, JsonPath::new(), 0, &handler ) {
    Ok(()) => (),
    Err(err) => { eprintln!("ending event reading {err:?}") },
  }
}
