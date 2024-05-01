/*!
This is a json path, ie an ordered set of steps,
where each step is either a key name or an index.
It must be optimised for add/remove the last element,
and cloning should be cheap.

`rpds::Vector` meets those requirements.
*/

/// The type for Index elements of a json path.
///
/// Highly unlikely the number of elements in a json array will exceed
/// 2^64 ie 18,446,744,073,709,551,616
type IndexInteger = u64;

/// One step in the path, which is either a tag name, or an integer index.
#[derive(Debug,Clone,Ord,PartialEq,Eq,PartialOrd)]
pub enum Step {
  Key(String),
  Index(IndexInteger),
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

// So we can offset an index into an array in both directions
type IndexOffset = i64;
impl std::ops::Add<IndexOffset> for Step
{
  type Output = Self;

  fn add(self, offset : IndexOffset) -> Self {
    match &self {
      Step::Key(v) => panic!("{v} is not an integer"),
      Step::Index(v) => {
        let (new_pos, overflow) = offset.overflowing_add_unsigned(*v);
        if overflow {panic!("offset {offset} from {v} is overflow")};
        Step::Index(new_pos as IndexInteger)
      }
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

impl From<IndexInteger> for Step {
  fn from(s: IndexInteger) -> Self { Self::Index(s) }
}

// These are all effectively AsRef
// But E0119 prevents implementing them using a trait.
impl From<&str> for Step {
  fn from(s: &str) -> Self { Self::Key(s.into()) }
}

impl From<String> for Step {
  fn from(s: String) -> Self { Self::Key(s) }
}

impl From<&String> for Step {
  fn from(s: &String) -> Self { Self::Key(s.into()) }
}

impl From<std::borrow::Cow<'_, str>> for Step {
  fn from(value: std::borrow::Cow<'_, str>) -> Self { Self::Key(value.into()) }
}

impl From<&std::borrow::Cow<'_, str>> for Step {
  #[allow(clippy::suspicious_to_owned)] // compiler fails unless to_owned is called
  fn from(value: &std::borrow::Cow<'_, str>) -> Self { Self::Key(value.to_owned().into()) }
}

// https://docs.rs/rpds/latest/rpds/list/struct.List.html
// type JsonPath = rpds::List<Step>;
pub type JsonPath = rpds::Vector<Step>;
