// type StrCon<T> = std::rc::Rc<T>;
type StrCon<T> = Box<T>;

// trait PosReader : std::io::BufRead + std::io::Seek {}
// trait PosReader : std::io::BufRead {}

fn make_readable(maybe_readable_args : &[String]) -> StrCon<dyn std::io::BufRead> {
  // use std::io::Read;
  match &maybe_readable_args[..] {
    [] => StrCon::new(std::io::stdin().lock()),
    [arg_fn] => {
      let file = std::fs::File::open(arg_fn).expect("cannot open file {arg_fn}");
      StrCon::new(std::io::BufReader::new(file))
    }
    _ => panic!("too many args {maybe_readable_args:?}")
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
  pub struct SendPath(pub JsonPath);

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

  // a tree path as sent by the streaming parser to a handler of some kind, along with its leaf value.
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

#[allow(dead_code)]
#[derive(Debug)]
enum Event<V> {
  Path(u64,SendPath),
  Value(SendPath,V),
  Finished,
}

trait Sender<T> {
  type SendError;
  fn send(&mut self, t: T) -> Result<(), Self::SendError>;
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
#[allow(unused_macros)]
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

// This traverses/handles the incoming json stream events.
//
// Rally just becomes a place to hang match_path and maybe_send_value without
// threading those functions through the JsonEvent handlers. Effectively it's a
// visitor with accept = match_path and visit = maybe_send_value
trait Handler {
  // value contained by Event
  // type V where Self::V : AsRef<&'a [u8]>;
  type V;

  fn match_path(&self, path : &JsonPath) -> bool;

  // default implementation that does nothing and returns OK
  fn maybe_send_value<Snd : Sender<Event<Self::V>>>(&self, path : &JsonPath, ev : &json_event_parser::JsonEvent, tx : &mut Snd) -> Result<(),Snd::SendError>;

  fn array<Snd : Sender<Event<Self::V>>>(&self, jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &mut Snd ) -> Result<(),Snd::SendError> {
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

  fn object<Snd : Sender<Event<Self::V>>>(&self, jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &mut Snd )
  -> Result<(),Snd::SendError> {
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

  fn value<Snd : Sender<Event<Self::V>>>(&self, jev : &mut JsonEvents, parents : Parents, depth : u64, tx : &mut Snd )
  -> Result<(),Snd::SendError> {
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

// write each leaf value to a separate file for its path
// a la the Shredder algorithm in Dremel paper
struct ShredWriter {
  // files : std::cell::RefCell<std::collections::hash_map::HashMap<String, std::fs::File>>
  dir : std::path::PathBuf,
  ext : String,
  files : std::collections::hash_map::HashMap<std::path::PathBuf, std::fs::File>,
}

impl ShredWriter {
  fn new(dir : &std::path::Path) -> Self {
    Self { dir: dir.to_path_buf(), files: std::collections::hash_map::HashMap::new(), ext: "mpk".into() }
  }

  // this converts a path in the form images/23423/image_name
  // to images.image.mpk
  // which basically means stripping out all Index components
  fn filename_of_path(dir : &std::path::PathBuf, send_path : &jsonpath::SendPath, ext : &String) -> std::path::PathBuf {
    let jsonpath::SendPath(steps) = send_path;
    let steps = steps
      .iter()
      // strip index parts
      .filter_map(|step| {
        match step {
          Step::Key(step) => Some(step.to_string()),
          Step::Index(_) => None,
        }
      })
      .collect::<Vec<String>>();
    let dotpath = steps.join(".");
    let mut filename = std::path::PathBuf::from(dotpath);
    filename.set_extension(ext);
    dir.join(filename)
  }

  fn find_or_create<'a>(&'a mut self, send_path : &jsonpath::SendPath) -> &'a std::fs::File {
    let filename = Self::filename_of_path(&self.dir, send_path, &self.ext);
    if self.files.contains_key(&filename) {
      self.files.get(&filename).unwrap()
    } else {
      eprintln!("new filename {filename:?}");
      let file = std::fs::File::create(&filename).unwrap();
      // by definition this is a None
      if let Some(_) = self.files.insert(filename.clone(), file) {
        panic!("oops with {filename:?}")
      }
      self.find_or_create(send_path)
    }
  }

  // receives events from the streaming parser
  fn write_msgpack_value(&mut self, ev : &Event<Vec<u8>>)
  {
    match ev {
      Event::Path(_depth,_path) => (),
      Event::Value(send_path,v) =>
      {
        let mut file = self.find_or_create(send_path);
        use std::io::Write;
        file.write_all(v).unwrap();
      },
      Event::Finished => (),
    }
  }
}

impl Sender<Event<Vec<u8>>> for ShredWriter {
  type SendError = ();

  fn send(&mut self, ev: Event<Vec<u8>>) -> Result<(), Self::SendError> {
    Ok(self.write_msgpack_value(&ev))
  }
}

struct MsgPacker();

impl MsgPacker {
  fn new() -> Self {
    Self()
  }
}

impl Handler for MsgPacker {
  type V = Vec<u8>;

  // filters events from the streaming parser
  fn match_path(&self, json_path : &JsonPath) -> bool {
    // need the &Step ref for nicer matching below
    let json_path = json_path.iter().collect::<Vec<&Step>>();

    // This is pretty horrible. Maybe a DSL would be nicer.
    match &json_path[..] {
      // ie images/xxx
      [&Step::Key(ref v), &Step::Index(ref _index), &Step::Key(ref _leaf_name)] => &v[..] == "images",
      _ => false
    }
  }

  // encode values as MessagePack, then send to shredder
  fn maybe_send_value<Snd : Sender<Event<Self::V>>>(&self, path : &JsonPath, &ev : &json_event_parser::JsonEvent, tx : &mut Snd)
  -> Result<(),Snd::SendError> {
    use json_event_parser::JsonEvent::*;
    let _ = match ev {
      String(v) => if self.match_path(&path) {
        let mut buf = vec![];
        match rmp::encode::write_str(&mut buf, &v) {
          Ok(()) => tx.send(Event::Value(SendPath::from(path),buf)),
          Err(err) => panic!("msgpack error {err}"),
        }
      } else {
        Ok(())
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
        Ok(())
      }

      Boolean(v) => if self.match_path(&path) {
        let mut buf = vec![];
        match rmp::encode::write_bool(&mut buf, v) {
          Ok(()) => tx.send(Event::Value(SendPath::from(path), buf)),
          Err(err) => panic!("msgpack error {err}"),
        }
      } else {
        Ok(())
      }

      Null => if self.match_path(&path) {
        let mut buf = vec![];
        match rmp::encode::write_nil(&mut buf) {
          Ok(()) => tx.send(Event::Value(SendPath::from(path), buf)),
          Err(err) => panic!("msgpack error {err}"),
        }
      } else {
        Ok(())
      }

      _ => todo!(),
    };

    Ok(())
  }
}


#[allow(dead_code, unused_mut, unused_variables)]
fn show_jq_paths() {
  let args = std::env::args().collect::<Vec<String>>();
  let istream = make_readable(&args[..]);
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

fn shred(dir : &std::path::PathBuf, maybe_readable_args : &[String]) {
  let istream = make_readable(maybe_readable_args);
  let mut jev = JsonEvents::new(istream);

  let mut writer = ShredWriter::new(&dir);
  let visitor = MsgPacker::new();

  match visitor.value(&mut jev, JsonPath::new(), 0, &mut writer ) {
    Ok(()) => (),
    Err(err) => { eprintln!("ending event reading {err:?}") },
  }
}

fn main() {
  let args = std::env::args().collect::<Vec<String>>();
  match &args[..] {
    [_] => panic!("you must provide data dir for files"),
    [_, dir, rst@..] => shred(&std::path::PathBuf::from(dir), rst),
    _ => panic!("only one data dir needed"),
  }
}
