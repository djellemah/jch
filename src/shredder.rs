// write each leaf value to a separate file for its path
// a la the Shredder algorithm in Dremel paper
use crate::handler::Handler;
use crate::jsonpath::*;
use crate::sender::*;
use crate::sendpath::SendPath;

pub struct ShredWriter<V> {
  dir : std::path::PathBuf,
  ext : String,
  files : std::collections::hash_map::HashMap<std::path::PathBuf, std::fs::File>,
  // only exists so rust doesn't erase V
  _event_marker : std::marker::PhantomData<V>,
}

impl<V> ShredWriter<V>
{
  pub fn new<S,P>(dir : P, ext : S)
  -> Self
  where
    S : AsRef<str> + std::fmt::Display,
    P : AsRef<std::path::Path>
  {
    Self {
      dir: std::path::PathBuf::from(dir.as_ref()),
      files: std::collections::hash_map::HashMap::new(),
      ext: ext.to_string(),
      _event_marker : std::marker::PhantomData,
    }
  }

  // this converts a path in the form images/23423/image_name
  // to images.image.mpk
  // which basically means stripping out all Index components
  //
  // TODO this is critical path for every single leaf that will be written. So it must be fast.
  // So probably the best way to do that is to skip the intermediate assignment of
  // 'steps' and just append directly to the (dir : Pathbuf)
  fn filename_of_path(dir : &std::path::PathBuf, send_path : &crate::sendpath::SendPath, ext : &String) -> std::path::PathBuf {
    // probably at least one index will be dropped, but we'll definitely need
    // space for the extension (ie +1).
    let mut steps : Vec<&str> = Vec::with_capacity(send_path.0.len() + 1);
    for step in send_path.0.iter() {
      if let Step::Key(step) = step { steps.push(step.as_str()) }
    }
    // append extension
    steps.push(ext);
    // construct the filename
    let filename = steps.join(".");
    // append filename to pathname
    dir.join(filename)
  }

  // find or create a given file for the jsonpath
  //
  // Self keeps a hashmap of
  //
  // PathBuf => File
  //
  // so it doesn't repeatedly reopen the same files.
  fn find_or_create<'a>(&'a mut self, send_path : &crate::sendpath::SendPath) -> &'a std::fs::File {
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
      // This only happens the first time the file is created,
      // so the extra lookup has little impact on the normal case.
      self.find_or_create(send_path)
    }
  }
}

impl ShredWriter<Vec<u8>>
{
  // receives events from the streaming parser
  pub fn write_msgpack_value<'a>(&mut self, ev : &Event<Vec<u8>>)
  {
    match ev {
      Event::Path(_depth,_path) => (),
      Event::Value(send_path,v) =>
      {
        let mut file = self.find_or_create(send_path);
        use std::io::Write;
        file.write_all(&v).unwrap();
      },
      Event::Finished => (),
      &Event::Error(_) => todo!(),
    }
  }
}

impl ShredWriter<&Vec<u8>>
{
  // receives events from the streaming parser
  pub fn write_msgpack_value<'a>(&mut self, ev : &Event<&Vec<u8>>)
  {
    match ev {
      Event::Path(_depth,_path) => (),
      Event::Value(send_path,v) =>
      {
        let mut file = self.find_or_create(send_path);
        use std::io::Write;
        file.write_all(&v).unwrap();
      },
      Event::Finished => (),
      &Event::Error(_) => todo!(),
    }
  }
}

impl ShredWriter<&[u8]>
{
  // receives events from the streaming parser
  fn write_msgpack_value<'a>(&mut self, ev : &Event<&[u8]>)
  {
    match ev {
      Event::Path(_depth,_path) => (),
      Event::Value(send_path,v) =>
      {
        let mut file = self.find_or_create(send_path);
        use std::io::Write;
        file.write_all(&v).unwrap();
      },
      Event::Finished => (),
      &Event::Error(_) => todo!(),
    }
  }
}

impl Sender<Event<&[u8]>> for ShredWriter<&[u8]> {
  type SendError = ();

  fn send<'a>(&mut self, ev: &'a Event<&'a [u8]>) -> Result<(), Self::SendError> {
    Ok(self.write_msgpack_value(&ev))
  }
}

impl Sender<Event<&Vec<u8>>> for ShredWriter<&Vec<u8>> {
  type SendError = ();

  fn send<'a>(&mut self, ev: &'a Event<&Vec<u8>>) -> Result<(), Self::SendError> {
    Ok(self.write_msgpack_value(&ev))
  }
}

impl Sender<Event<Vec<u8>>> for ShredWriter<Vec<u8>> {
  type SendError = ();

  fn send<'a>(&mut self, ev: &'a Event<Vec<u8>>) -> Result<(), Self::SendError> {
    Ok(self.write_msgpack_value(&ev))
  }
}

pub struct MsgPacker();

impl MsgPacker {
  pub fn new() -> Self {
    Self()
  }

  fn encode_to_msgpack<'a>(path : &JsonPath, ev : &json_event_parser::JsonEvent, buf : &'a mut Vec<u8>)
  -> Event<&'a Vec<u8>>
  {
    use json_event_parser::JsonEvent::*;

    match ev {
      &String(v) => {
        match rmp::encode::write_str(buf, &v) {
          Ok(()) => Event::Value(SendPath::from(path),buf),
          Err(err) => panic!("msgpack error {err}"),
        }
      }

      &Number(v) => {
        let number_value : serde_json::Number = match serde_json::from_str(v) {
          Ok(n) => n,
          Err(msg) => panic!("{v} appears to be not-a-number {msg}"),
        };

        // NOTE trying to wrap this in a Result instead of panic! causes trouble
        // because Event<&'a mut Vec<u8>? is not coerceable to Event<&'a Vec<u8>>
        // despite coercion rules ¯\_(/")_/¯
        // So Event enum then needs the Error(_) item
        if number_value.is_u64() {
          match rmp::encode::write_uint(buf, number_value.as_u64().unwrap()) {
            Ok(_) => Event::Value(SendPath::from(path), buf),
            Err(err) => Event::Error(format!("{err:?}")),
          }
        } else if number_value.is_i64() {
          match rmp::encode::write_sint(buf, number_value.as_i64().unwrap()) {
            Ok(_) => Event::Value(SendPath::from(path), buf),
            Err(err) => Event::Error(format!("{err:?}")),
          }
        } else if number_value.is_f64() {
          match rmp::encode::write_f64(buf, number_value.as_f64().unwrap()) {
            Ok(()) => Event::Value(SendPath::from(path), buf),
            Err(err) => Event::Error(format!("{err:?}")),
          }
        } else {
          panic!("wut!?")
        }
      }

      &Boolean(v) => {
        match rmp::encode::write_bool(buf, v) {
          Ok(()) => Event::Value(SendPath::from(path), buf),
          Err(err) => panic!("msgpack error {err}"),
        }
      }

      Null => {
        match rmp::encode::write_nil(buf) {
          Ok(()) => Event::Value(SendPath::from(path), buf),
          Err(err) => panic!("msgpack error {err}"),
        }
      }

      _ => todo!(),
    }
  }
}

impl Handler for MsgPacker {
  // V is the type of the data that Event contains
  // TODO both of this work with only the need to borrow buf or not.
  // type V<'l> = &'l[u8];
  type V<'l> = &'l Vec<u8>;

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
  fn maybe_send_value<'a, Snd>(&self, path : &JsonPath, ev : &json_event_parser::JsonEvent, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<MsgPacker as Handler>::V<'_>>>>::SendError>
  // the `for` is critical here because 'x must have a longer lifetime than 'a but a shorter lifetime than 'l
  where Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
    if !self.match_path(&path) { return Ok(()) }
    let mut buf = vec![];
    let send_event = Self::encode_to_msgpack(path, ev, &mut buf);
    if let Err(_msg) = tx.send(&send_event) {
      // TODO Sender::Event::<V> does not implement Display or Debug so we can't use it here.
      panic!("could not send event {ev:?}");
    }
    Ok(())
  }
}
