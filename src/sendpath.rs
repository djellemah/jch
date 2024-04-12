use crate::jsonpath::JsonPath;

#[derive(Debug,Ord,PartialOrd,Eq,PartialEq,Clone)]
pub struct SendPath(pub JsonPath);

impl From<&JsonPath> for SendPath {
  fn from(jsonpath : &JsonPath) -> Self {
    Self(jsonpath.clone())
  }
}

// a tree path as sent by the streaming parser to a handler of some kind, along with its leaf value.
// struct SendPath(Vec<Step>);
// impl From<&JsonPath> for SendPath {
//   fn from(path_list : &JsonPath) -> Self {
//     let steps = path_list.iter().map(std::clone::Clone::clone).collect::<Vec<Step>>();
//     // steps.reverse(); for list
//     Self(steps)
//   }
// }

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

// #[allow(dead_code)]
#[derive(Debug)]
pub enum Event<V> {
  Path(u64,SendPath),
  Value(SendPath,V),
  Finished,
  Error(String),
}

pub trait Sender<T> {
  type SendError;
  fn send<'a>(&mut self, t: &'a T) -> Result<(), Self::SendError>;
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
    $tx.send( &Event::Path(0, SendPath::from($parents)) )
  };
  ($tx:ident,0,$parents:expr) => {
    $tx.send( &Event::Path(0, SendPath::from($parents)) )
  };
  ($tx:ident,$depth:ident,&$parents:expr) => {
    $tx.send( &Event::Path(0, SendPath::from($parents)) )
  };
  ($tx:ident,$depth:ident,$parents:expr) => {
    $tx.send( &Event::Path(0, SendPath::from($parents)) )
  };
}
