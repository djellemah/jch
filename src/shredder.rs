/*!
This writes out a file for each path, where indexes are removed from the path.
Each file contains all the values from that path, in order.
*/

use std::sync::Arc;

use crate::parser;
use crate::handler::Handler;
use crate::jsonpath::*;
use crate::sender;
use crate::sendpath::SendPath;
use crate::parser::JsonEvent;

pub struct ShredWriter<V,W> {
  dir : std::path::PathBuf,
  ext : String,
  files : std::collections::hash_map::HashMap<std::path::PathBuf, std::fs::File>,
  // only exists so rust doesn't erase V
  _event_marker : std::marker::PhantomData<V>,
  _ewent_marker : std::marker::PhantomData<W>,
}

impl<V,W> ShredWriter<V, W>
{
  pub fn new<S,P>(dir : P, ext : S)
  -> Self
  where
    S : AsRef<str> + std::fmt::Display,
    P : AsRef<std::path::Path>
  {
    let dir = std::path::PathBuf::from(dir.as_ref());
    if !dir.is_dir() {
      println!("{dir:?} must be a directory.");
      // Must use exit here otherwise the other thread doesn't shut down.
      std::process::exit(1)
    }
    Self {
      dir,
      files: std::collections::hash_map::HashMap::new(),
      ext: ext.to_string(),
      _event_marker : std::marker::PhantomData,
      _ewent_marker : std::marker::PhantomData,
    }
  }

  /// find or create a given file for the jsonpath
  ///
  /// Self keeps a hashmap of
  ///
  /// `PathBuf => File`
  ///
  /// so it doesn't repeatedly reopen the same files.
  fn find_or_create<'a>(&'a mut self, send_path : &crate::sendpath::SendPath) -> &'a std::fs::File {
    let pathname = self.dir.join(filename_of_path(send_path, &self.ext));
    if self.files.contains_key(&pathname) {
      self.files.get(&pathname).unwrap()
    } else {
      // expect here because by now the filename should have valid characters, and other errors are fatal anyway.
      #[allow(clippy::expect_fun_call)]
      let file = std::fs::File::create(&pathname).expect(format!("error for path {pathname:?}").as_str());
      // by definition this is a None
      if self.files.insert(pathname.clone(), file).is_some() {
        panic!("oops with {pathname:?}")
      }
      // This only happens the first time the file is created,
      // so the extra lookup has little impact on the normal case.
      self.find_or_create(send_path)
    }
  }
}

impl<V: AsRef<[u8]> + std::fmt::Debug, W> ShredWriter<V, W>
{
  /// Writes events from our event source, whose ultimate source is a streaming parser.
  pub fn write_msgpack_value(&mut self, ev : &sender::Event<V>)
  {
    use sender::Event;
    match ev {
      Event::Value(send_path,v) =>
      {
        let mut file = self.find_or_create(send_path);
        use std::io::Write;
        // is a Vec inside an Arc, hence the double as_ref
        file.write_all(v.as_ref().as_ref()).unwrap();
      },
      Event::Path(_depth,_path) => todo!("Event::Path"),
      Event::Finished => todo!("Event::Finished"),
      Event::Error(_,_) => todo!("Event::Error"),
    }
  }
}

impl<V : AsRef<[u8]> + std::fmt::Debug, W: Send + std::ops::Deref<Target = sender::Event<V>>> sender::Sender<sender::Event<V>, W> for ShredWriter<V,W> {
  #[allow(clippy::unit_arg)]
  fn send(&mut self, ev: W) -> Result<(), Box<dyn std::error::Error>> {
    Ok(self.write_msgpack_value(&ev))
  }
}

/// convert the given json event to a sender event containing messagepack in its buffer
fn encode_to_msgpack
<'a, 'b, Path: 'a, Stringish>
(path : &'a Path, ev : &'b JsonEvent<Stringish>)
-> sender::Event<Vec<u8>>
where
  Stringish : 'b + AsRef<[u8]> + AsRef<str> + std::fmt::Display,
  crate::sendpath::SendPath : for<'sp> From<&'sp Path>
{
  // store msgpack bytes in here
  let mut buf = vec![];

  use sender::Event;
  match ev {
    JsonEvent::String(v) => {
      match rmp::encode::write_str (&mut buf, v.as_ref() ) {
        Ok(()) => Event::Value(SendPath::from(path), buf),
        Err(err) => Event::Error(path.into(), format!("msgpack error {err:?}")),
      }
    }

    JsonEvent::Number(v) => {
      let number_value : serde_json::Number = match serde_json::from_str(v.as_ref()) {
        Ok(n) => n,
        Err(msg) => return Event::Error(path.into(), format!("{v} appears to be not-a-number {msg}")),
      };

      if number_value.is_u64() {
        match rmp::encode::write_uint(&mut buf, number_value.as_u64().unwrap()) {
          Ok(_) => Event::Value(SendPath::from(path), buf),
          Err(err) => Event::Error(path.into(), format!("msgpack error {err:?}")),
        }
      } else if number_value.is_i64() {
        match rmp::encode::write_sint(&mut buf, number_value.as_i64().unwrap()) {
          Ok(_) => Event::Value(SendPath::from(path), buf),
          Err(err) => Event::Error(path.into(), format!("msgpack error {err:?}")),
        }
      } else if number_value.is_f64() {
        match rmp::encode::write_f64(&mut buf, number_value.as_f64().unwrap()) {
          Ok(()) => Event::Value(SendPath::from(path), buf),
          Err(err) => Event::Error(path.into(), format!("msgpack error {err:?}")),
        }
      } else {
        panic!("wut!?")
      }
    }

    JsonEvent::Boolean(v) => {
      match rmp::encode::write_bool(&mut buf, *v) {
        Ok(()) => Event::Value(SendPath::from(path), buf),
        Err(err) => Event::Error(path.into(), format!("msgpack error {err:?}")),
      }
    }

    JsonEvent::Null => {
      match rmp::encode::write_nil(&mut buf) {
        Ok(()) => Event::Value(SendPath::from(path), buf),
        Err(err) => Event::Error(path.into(), format!("msgpack error {err:?}")),
      }
    }

    _ => todo!(),
  }
}

pub struct MsgPacker();

impl Default for MsgPacker {
  fn default() -> Self {
    Self::new()
  }
}

impl MsgPacker {
  pub fn new() -> Self {
    Self()
  }
}

type SendValue = Vec<u8>;
type SendEvent = sender::Event<SendValue>;

use crate::sender::Sender;

impl<'l> Handler<'l, SendValue, Arc<SendEvent>, dyn Sender<SendEvent,Arc<SendEvent>> + 'l> for MsgPacker {
  // TODO handle both ref to buffer and buffer

  // filters events from the streaming parser
  fn match_path(&self, _json_path : &JsonPath) -> bool {
    true
  }

  // encode values as MessagePack, then send to shredder
  fn maybe_send_value(&self, path : &JsonPath, ev : JsonEvent<String>, tx : &mut (dyn Sender<SendEvent,Arc<SendEvent>> + 'l))
  -> Result<(), Box<dyn std::error::Error>>
  {
    if !self.match_path(path) { return Ok(()) }
    let send_event = encode_to_msgpack::<JsonPath,String>(path, &ev);
    // OPT must this really be in a box?
    let () = tx
      .send(Arc::new(send_event))
      .unwrap_or_else(|err| panic!("could not send event {ev:?} because {err:?}"));
    Ok(())
  }
}

impl AsRef<[u8]> for sender::Event<Vec<u8>> {
  fn as_ref(&self) -> &[u8] {
    use sender::Event;
    match self {
      Event::Path(_, _send_path) => todo!(),
      Event::Value(_send_path, _) => todo!(),
      Event::Finished => todo!(),
      Event::Error(_send_path, _) => todo!(),
    }
  }
}

pub fn shred<Stringish>(dir : &std::path::PathBuf, maybe_readable_args : &[Stringish])
where Stringish : AsRef<str> + AsRef<std::path::Path> + std::fmt::Debug
{
  let istream = crate::make_readable(maybe_readable_args);
  let mut jevstream = parser::JsonEventParser::new(istream);

  // write events as Dremel-style record shred columns
  let mut writer = crate::shredder::ShredWriter::new(dir, "mpk");

  // serialisation format for columns
  let visitor = MsgPacker::new();

  visitor
    .value(&mut jevstream, JsonPath::new(), 0, &mut writer )
    .unwrap_or_else(|err| eprintln!("ending event reading because {err:?}") );
}

// T = serde_json::Value, for example
pub fn channel_shred<S>(dir : &std::path::Path, maybe_readable_args : &[S])
where S : AsRef<str> + AsRef<std::path::Path> + std::fmt::Debug
{
  // Create ShredWriter first, because it might want to stop things.
  let mut writer : ShredWriter<Vec<u8>,Arc<Vec<u8>>> = ShredWriter::new(dir, "mpk");

  // Crossbeam Channel
  let (mut tx, rx) =  {
    // this seems to be about optimal wrt performance
    const CHANNEL_SIZE : usize = 8192;
    crossbeam::channel::bounded::<sender::NonWrap<sender::Event<JsonEvent<String>>>>(CHANNEL_SIZE)
  };

  // consumer thread
  let cons_thr = {
    std::thread::Builder::new().name("jch recv".into()).spawn(move || {
      // use crate::channel::Consumer;
      while let Ok(event) = rx.recv() {
        use sender::Event;
        let msgpacked_event = match event.as_ref() {
          Event::Value(path,jev) => encode_to_msgpack::<SendPath,String>(path, jev),
          Event::Error(path, msg) => {println!("{msg} at path '{path}'"); continue},
          Event::Finished => break,
          err => todo!("{err:?}"),
        };

        writer.write_msgpack_value(&msgpacked_event)
      }
    }).expect("cannot create recv thread")
  };

  // jump through hoops so cons_thr join will work
  {
    let istream = crate::make_readable(maybe_readable_args);
    let mut jevstream = parser::JsonEventParser::new(istream);

    // This will send `sender::Event<plain::JsonEvent>` over the channel
    use crate::plain::Plain;
    let visitor = Plain(|_| true, std::marker::PhantomData);

    visitor.value(&mut jevstream, JsonPath::new(), 0, &mut tx).unwrap_or_else(|_| println!("uhoh"));
    // tx dropped automatically here, so channel gets closed
  }

  // done with the weird hoops
  cons_thr.join().unwrap();
}


/**
Converts a path in the form `images/23423/image_name` to
`images.image_name.mpk`. Which basically means stripping out all Index
components.

It's on the critical path for every single leaf that will be written. So it must
be fast. That, along with the need to detect a potentially empty filename, are
the drivers behind the fancy iterator chain.
*/
fn filename_of_path<'a>(send_path : &'a crate::sendpath::SendPath, ext : &'a str) -> std::path::PathBuf {
  let mut steps = send_path.0.iter().filter_map(|step|
    if let Step::Key(step) = step {
      Some(step.as_str())
    } else {
      None
    }
  );

  // find a sensible default if the path is empty
  let first_step = steps.by_ref().next().unwrap_or("_");

  // Build the path as elt1.elt2.elt3.ext, and then convert to return type, ie PathBuf
  use std::iter::once;
  once(first_step)
    .chain(steps)
    .chain(once(ext))
    .intersperse(".")
    .collect::<String>()
    .replace([' ','/'],"_")
    .into()
}

#[cfg(test)]
mod test_filename_of_path {
  use super::*;
  use std::path::PathBuf;

  #[test]
  fn normal() {
    let send_path = SendPath(vec![Step::Index(0), Step::Key("uno".into()), Step::Key("duo".into()), Step::Key("tre".into())]);
    let ext = "wut";
    let path = super::filename_of_path(&send_path, ext);
    assert_eq!(path, PathBuf::from("uno.duo.tre.wut"));
  }

  #[test]
  fn empty() {
    let send_path = SendPath(vec![]);
    let ext = "wut";
    let path = super::filename_of_path(&send_path, ext);
    assert_eq!(path, PathBuf::from("_.wut"));
  }

  #[test]
  fn index_only() {
    let send_path = SendPath(vec![Step::Index(0)]);
    let ext = "wut";
    let path = super::filename_of_path(&send_path, ext);
    assert_eq!(path, PathBuf::from("_.wut"));
  }

  #[test]
  fn several_leading_index() {
    let send_path = SendPath(vec![Step::Index(0),Step::Index(0),Step::Index(0)]);
    let ext = "wut";
    let path = super::filename_of_path(&send_path, ext);
    assert_eq!(path, PathBuf::from("_.wut"));
  }

  #[test]
  fn bad_chars() {
    // space and /
    let send_path = SendPath(vec![Step::Key("this is a bad/dangerous path".into())]);
    let ext = "wut";
    let path = super::filename_of_path(&send_path, ext);
    assert_eq!(path, PathBuf::from("this_is_a_bad_dangerous_path.wut"));
  }
}
