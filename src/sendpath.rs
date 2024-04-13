// Basically this implements a JsonPath that's optimised for sending over a
// channel without excessive copying.

use crate::jsonpath::JsonPath;
use crate::jsonpath::Step;

mod like_jsonpath {
  // rpds needs a special constructor for this, so that we don't run into trouble
  // with RcK in archery being not thread-safe.
  use super::JsonPath;

  #[derive(Debug,Ord,PartialOrd,Eq,PartialEq,Clone)]
  pub struct SendPath(pub JsonPath);

  impl From<&JsonPath> for SendPath {
    fn from(jsonpath : &JsonPath) -> Self {
      Self(jsonpath.clone())
    }
  }
}

// a tree path as sent by the streaming parser to a handler of some kind, along with its leaf value.
#[derive(Debug,Clone)]
pub struct SendPath(pub Vec<Step>);

impl From<&JsonPath> for SendPath {
  fn from(path_list : &JsonPath) -> Self {
    let steps = path_list.iter().map(std::clone::Clone::clone).collect::<Vec<Step>>();
    // steps.reverse(); for list
    Self(steps)
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
