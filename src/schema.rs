use crate::Handler;
use crate::Sender;
use crate::JsonPath;
use crate::Event;

// tree is a map of path => [(type, count)]

#[derive(Debug,Clone)]
pub enum NumberType {
  // max
  Unsigned(u64),
  // min max
  Signed(i64, i64),
  // min max
  Float(f64, f64),
}

impl PartialEq for NumberType {
  fn eq(&self, rhs: &NumberType) -> bool {
    match (self, rhs) {
      (NumberType::Unsigned(a), NumberType::Unsigned(b)) => a == b,
      (NumberType::Signed(ax, an), NumberType::Signed(bx, bn)) => ax == bx && an == bn,
      (NumberType::Float(ax, an), NumberType::Float(bx, bn)) => ax == bx && an == bn,
        _ => false
    }
  }
}

impl Eq for NumberType{}

impl std::hash::Hash for NumberType {
  fn hash<H>(&self, hsh: &mut H) where H: std::hash::Hasher {
    match self {
      NumberType::Unsigned(n) => hsh.write_u64(*n),
      NumberType::Signed(nx, nn) => { hsh.write_i64(*nx); hsh.write_i64(*nn) },
      NumberType::Float(nx, nn) => {
        let xbytes : [u8 ; 8] = unsafe { std::mem::transmute(nx) };
        let nbytes : [u8 ; 8] = unsafe { std::mem::transmute(nn) };
        hsh.write(&xbytes);
        hsh.write(&nbytes)
      },
    }
  }
}

#[derive(Debug,Clone,Eq,PartialEq,Hash)]
pub enum SchemaType {
  // max_len
  String(u64),
  Number(NumberType),
  Boolean,
  Null,
  Unknown(String),
}

#[derive(Debug,Clone,Eq,PartialEq)]
#[allow(dead_code)]
struct Leaf {
  kind : SchemaType,
  count : std::cell::RefCell<u64>,
}

impl Leaf {
  fn new(kind : SchemaType) -> Self {
    Self{ kind: kind.clone(), count: std::cell::RefCell::new(1)}
  }
}

impl std::fmt::Display for Leaf {
  fn fmt(&self, f : &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    let kind = &self.kind;
    let count = &self.count;
    write!(f, "{kind:?}:{}", count.borrow())
  }
}

impl std::hash::Hash for Leaf {
  fn hash<H>(&self, hsh: &mut H) where H: std::hash::Hasher {
    self.kind.hash(hsh);
    self.count.borrow().hash(hsh);
  }
}


#[derive(Debug,Clone,Ord,PartialEq,Eq,PartialOrd)]
pub enum Step {
  Key(String),
  Index,
}

impl std::fmt::Display for Step {
  fn fmt(&self, f : &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    match &self {
      Step::Key(v) => write!(f, "{v}"),
      Step::Index => write!(f, "[]"),
    }
  }
}

#[derive(Debug,Ord,PartialOrd,Eq,PartialEq)]
struct SchemaPath(Vec<Step>);

impl std::fmt::Display for SchemaPath {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let string_parts = self.0.iter().map(ToString::to_string).collect::<Vec<String>>();
    let repr = string_parts.join("/");

    write!(f,"{repr}")
  }
}

type LeafPaths = std::collections::BTreeMap<SchemaPath, std::collections::HashSet<Leaf>>;

pub struct SchemaCalculator();

impl SchemaCalculator {
  pub fn new() -> Self {Self()}

  fn collect_type<'a>(&self, _path : &JsonPath, ev : &json_event_parser::JsonEvent)
  -> SchemaType
  {
    use json_event_parser::JsonEvent;

    match ev {
      &JsonEvent::String(v) => {
        if v == "NaN" {
          SchemaType::Number(NumberType::Float(f64::NAN,f64::NAN))
        } else {
          SchemaType::String(v.len() as u64)
        }
      }

      &JsonEvent::Number(v) => {
        let number_value : serde_json::Number = match serde_json::from_str(v) {
          Ok(n) => n,
          Err(msg) => panic!("{v} appears to be not-a-number {msg}"),
        };

        if number_value.is_u64() {
          // TODO 0 must calculate max
          let n = number_value.as_u64().unwrap();
          SchemaType::Number(NumberType::Unsigned(n))
        } else if number_value.is_i64() {
          // TODO 0 must calculate max
          let i = number_value.as_i64().unwrap();
          SchemaType::Number(NumberType::Signed(i,i))
        } else if number_value.is_f64() {
          // TODO 0 must calculate max
          let f = number_value.as_f64().unwrap();
          SchemaType::Number(NumberType::Float(f,f))
        } else {
          SchemaType::Unknown(v.into())
        }
      }

      &JsonEvent::Boolean(_v) => {
        SchemaType::Boolean
      }

      JsonEvent::Null => {
        SchemaType::Null
      }

      ev => SchemaType::Unknown(format!("{ev:?}")),
    }
  }
}

impl Handler for SchemaCalculator {
  type V<'l> = SchemaType;

  // collect all paths
  fn match_path(&self, _json_path : &JsonPath) -> bool {true}

  // encode values as MessagePack, then send to shredder
  fn maybe_send_value<'a, Snd>(&self, path : &JsonPath, ev : &json_event_parser::JsonEvent, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<SchemaCalculator as Handler>::V<'_>>>>::SendError>
  // the `for` is critical here because 'x must have a longer lifetime than 'a but a shorter lifetime than 'l
  where Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
    if !self.match_path(&path) { return Ok(()) }
    let schema_type = self.collect_type(path, ev);
    match tx.send(&Event::Value(path.into(), schema_type)) {
        Ok(()) => Ok(()),
        Err(_err) => panic!("aaargh implement Debug for Sender<Event...>"),
    }
  }
}

// empty struct
#[allow(dead_code)]
#[derive(Debug)]
pub struct SchemaCollector {
  leaf_paths : LeafPaths
}

impl SchemaCollector {
  pub fn new() -> Self {
    Self {leaf_paths: LeafPaths::new()}
  }
}

impl std::fmt::Display for SchemaCollector {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error>
  {
    for (p,kinds) in &self.leaf_paths {
      const WIDTH : usize = 40;
      // because otherwise 50 width is applied to each element of k

      let kfmts = kinds
        .iter()
        .map(|k| format!("{k:WIDTH$}") )
        .collect::<Vec<String>>();

      let kfmt = match kinds.len() {
        0 => String::new(),
        1 => kfmts.into_iter().last().unwrap(), // no point creating another string here
        _ => format!("[{}]", kfmts.join(","))
      };

      write!(f, "{kfmt:35} {p}\n")?;
    };
    Ok(())
  }
}

impl Sender<Event<SchemaType>> for SchemaCollector {
  type SendError = ();

  fn send<'a>(&mut self, ev: &'a Event<SchemaType>) -> Result<(), Self::SendError> {
    match ev {
        Event::Path(_p, _v) => todo!(),
        Event::Value(p, schema_type) => {
          let path = p.0.iter().map(|step| {
            // replace all indexes in path with generic placeholder. Because we
            // want the schema not the full tree.
            match step {
                crate::Step::Key(v) => Step::Key(v.to_string()),
                crate::Step::Index(_) => Step::Index,
            }
          }).collect::<Vec<Step>>();
          let path = SchemaPath(path);

          match self.leaf_paths.get_mut(&path) {
            Some(leafs) => {
              // find the type in leafs
              use SchemaType::*;
              use NumberType::*;
              let kind = leafs.iter().find(|Leaf{kind: stored_kind, ..}| {
                match (schema_type, stored_kind) {
                  (String(_), String(_)) => true,
                  (Number(Unsigned(_)), Number(Unsigned(_))) => true,
                  (Number(Signed(_,_)), Number(Signed(_,_))) => true,
                  (Number(Float(_,_)), Number(Float(_,_))) => true,
                  (Boolean, Boolean) => true,
                  (Null, Null) => true,
                  _ => false,
                }
              });

              // either create a new type, or update the existing type with current counts and values
              match kind {
                Some(stored_kind) => {
                  let mut count = stored_kind.count.borrow_mut();
                  *count += 1;
                }
                None => { leafs.insert(Leaf::new(schema_type.clone())); }
              }

            },
            None => {
              let mut leafs = std::collections::HashSet::new();
              leafs.insert(Leaf::new(schema_type.clone()));
              self.leaf_paths.insert(path, leafs);
            }
          }
        }
        Event::Finished => todo!(),
        Event::Error(_) => todo!(),
    }
    Ok(())
  }
}

pub fn schema(jev : &mut crate::JsonEvents) {
  // collect and display schema of input
  let mut collector = SchemaCollector::new();

  // translate start/end streaming events to leaf types
  let visitor = SchemaCalculator::new();

  match visitor.value(jev, JsonPath::new(), 0, &mut collector ) {
    Ok(()) => println!("{collector}"),
    Err(err) => { eprintln!("ending event reading because {err:?}") },
  }
}
