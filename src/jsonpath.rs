// highly unlikely the number of elements in a json array will exceed
// 2^64 ie 18,446,744,073,709,551,616
type IndexInteger = u64;

// One step in the path, which is either a tag name, or an integer index.
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

impl From<&str> for Step {
  fn from(s: &str) -> Self { Self::Key(s.to_string()) }
}

impl From<IndexInteger> for Step {
  fn from(s: IndexInteger) -> Self { Self::Index(s) }
}

// https://docs.rs/rpds/latest/rpds/list/struct.List.html
// type JsonPath = rpds::List<Step>;
pub type JsonPath = rpds::Vector<Step>;
