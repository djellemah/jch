/*!
One way to view a tree is a map of path => value, or for its schema path => type.

This parses a json document and collects the type for each path, including some basic statistics.
*/

use std::cell::RefCell;

use crate::parser::JsonEvents;
use crate::handler::Handler;
use crate::sender::Sender;
use crate::jsonpath::JsonPath;
use crate::sender::Event;
use crate::parser::JsonEvent;

/*
tree is a map of path => [(type, count)]

NOTE this already sortof exists in serde_json with feature arbitrary_precision
enum N {
   PosInt(u64),
   /// Always less than zero.
   NegInt(i64),
   /// Always finite.
   Float(f64),
}
*/

/// The various kinds of json number, in numeric format.
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
      (NumberType::Signed(an, ax), NumberType::Signed(bn, bx)) => an == bn && ax == bx,
      (NumberType::Float(an, ax), NumberType::Float(bn, bx)) => an == bn && ax == bx,
       _ => false
    }
  }
}

impl Eq for NumberType{}

impl std::hash::Hash for NumberType {
  fn hash<H>(&self, hsh: &mut H) where H: std::hash::Hasher {
    match self {
      NumberType::Unsigned(n) => hsh.write_u64(*n),
      NumberType::Signed(nn, nx) => { hsh.write_i64(*nn); hsh.write_i64(*nx) },
      NumberType::Float(nn, nx) => {
        let nbytes : [u8 ; 8] = unsafe { std::mem::transmute(nn) };
        let xbytes : [u8 ; 8] = unsafe { std::mem::transmute(nx) };
        hsh.write(&nbytes);
        hsh.write(&xbytes)
      },
    }
  }
}

/// enum for the types in a schema.
#[derive(Debug,Clone,Eq,PartialEq,Hash)]
pub enum SchemaType {
  // max_len
  String(u64),
  Number(NumberType),
  Boolean,
  Null,
  Unknown(String),
}

/**
For each path in the tree, this stores the kind of value at this path, along
with statistical type data about how many times and what values are stored
there.
*/
#[derive(Debug,Clone,Eq,PartialEq)]
struct Leaf {
  kind : SchemaType,
  count : RefCell<u64>,
  // min/max length etc go here
  aggregate : RefCell<SchemaType>,
}

impl Leaf {
  fn new(kind : SchemaType) -> Self {
    Self{ kind: kind.clone(), count: RefCell::new(1), aggregate: RefCell::new(kind.clone())}
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
  /// Hash only the part that will be stable, and since count and aggregate are
  /// both RefCell, they won't be stable.
  fn hash<H>(&self, hsh: &mut H) where H: std::hash::Hasher {
    self.kind.hash(hsh);
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

pub struct EventConverter();

impl EventConverter {
  pub fn new() -> Self {Self()}

  fn collect_type<'a>(&self, _path : &JsonPath, ev : &JsonEvent<String>)
  -> SchemaType
  {
    // So the big question is: should this translation happen: in the parser thread; or in the processor thread?
    match ev {
      JsonEvent::String(v) => {
        if v == "NaN" {
          SchemaType::Number(NumberType::Float(f64::NAN,f64::NAN))
        } else {
          SchemaType::String(v.len() as u64)
        }
      }

      JsonEvent::Number(v) => {
        let number_value : serde_json::Number = match serde_json::from_str(&v) {
          Ok(n) => n,
          Err(msg) => panic!("{v} appears to be not-a-number {msg}"),
        };

        if number_value.is_u64() {
          let n = number_value.as_u64().unwrap();
          SchemaType::Number(NumberType::Unsigned(n))
        } else if number_value.is_i64() {
          let i = number_value.as_i64().unwrap();
          SchemaType::Number(NumberType::Signed(i,i))
        } else if number_value.is_f64() {
          let f = number_value.as_f64().unwrap();
          SchemaType::Number(NumberType::Float(f64::min(f,0.0),f64::max(f, 0.0)))
        } else {
          SchemaType::Unknown(v.to_string())
        }
      }

      JsonEvent::Boolean(_v) => SchemaType::Boolean,
      JsonEvent::Null => SchemaType::Null,
      ev => SchemaType::Unknown(format!("{ev:?}")),
    }
  }
}

impl Handler for EventConverter {
  type V<'l> = SchemaType;

  // collect all paths
  fn match_path(&self, _json_path : &JsonPath) -> bool {true}

  fn maybe_send_value<'a, Snd>(&self, path : &JsonPath, ev : &JsonEvent<String>, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<EventConverter as Handler>::V<'_>>>>::SendError>
  // the `for` is critical here because 'x must have a longer lifetime than 'a but a shorter lifetime than 'l
  where Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
    if !self.match_path(&path) { return Ok(()) }
    let schema_type = self.collect_type(path, ev);
    tx
      .send(Box::new(Event::Value(path.into(), schema_type)))
      .unwrap_or_else(|err| panic!("cannot send {ev:?} because {err:?}"));
    Ok(())
  }
}

type LeafKinds = std::collections::HashSet<Leaf>;
type LeafPaths = std::collections::BTreeMap<SchemaPath, LeafKinds>;

#[derive(Debug)]
pub struct SchemaCollector {
  leaf_paths : LeafPaths
}

impl SchemaCollector {
  pub fn new() -> Self {
    Self {leaf_paths: LeafPaths::new()}
  }

  // This is where we aggregate the types from the stream of incoming types
  fn process_event<'a>(&mut self, ev: &Event<SchemaType>) -> () {
    match ev {
        Event::Path(_p, _v) => todo!(),
        Event::Value(p, value_type) => {
          let path = p.0.iter().map(|step| {
            // replace all indexes in path with generic placeholder. Because we
            // want the schema not the full tree.
            match step {
              crate::jsonpath::Step::Key(v) => Step::Key(v.clone()),
              crate::jsonpath::Step::Index(_) => Step::Index,
            }
          }).collect::<Vec<Step>>();
          let path = SchemaPath(path);

          // leaf_paths is path => Set<Leaf>
          match self.leaf_paths.get_mut(&path) {
            Some(leaf_kinds) => {
              // find the current type in leaf_kinds
              use SchemaType::*;
              use NumberType::*;
              let kind_option = leaf_kinds.iter().find(|Leaf{kind: stored_kind, ..}| {
                match (value_type, stored_kind) {
                  (String(_), String(_)) => true,
                  (Number(Unsigned(_)), Number(Unsigned(_))) => true,
                  (Number(Signed(_,_)), Number(Signed(_,_))) => true,
                  (Number(Float(_,_)), Number(Float(_,_))) => true,
                  (Boolean, Boolean) => true,
                  (Null, Null) => true,
                  _ => false,
                }
              });

              // This is is now a particular SchemaType stored at leaf
              // either create a new type, or update the existing type with current counts and values
              match kind_option {
                Some(kind) => {
                  // increment count
                  *kind.count.borrow_mut() += 1;

                  // update the max/min and other aggregates here
                  // transfer values from value_type (ie the current leaf value) to aggregate (ie in the schema we're building)
                  let updated_aggregate_option = match (value_type,&*kind.aggregate.borrow()) {
                    (&String(val_n), &String(agg_n)) => Some(String(std::cmp::max(val_n,agg_n))),
                    (&Number(Unsigned(val_max)), &Number(Unsigned(agg_max))) => Some(Number(Unsigned(std::cmp::max(val_max,agg_max)))),
                    (&Number(Signed(val_min,val_max)), &Number(Signed(agg_min,agg_max))) => Some(Number(Signed(std::cmp::min(val_min,agg_min), std::cmp::max(val_max,agg_max)))),
                    (&Number(Float(val_min,val_max)), &Number(Float(agg_min,agg_max))) => Some(Number(Float(f64::min(val_min,agg_min), f64::max(val_max,agg_max)))),
                    _ => None, // because no aggregates are collected for other types, so no need to update anything
                  };

                  if let Some(updated_aggregate) = updated_aggregate_option {
                    kind.aggregate.replace(updated_aggregate);
                  }
                }
                None => { leaf_kinds.insert(Leaf::new(value_type.clone())); }
              }

            },
            None => {
              // There are as yet no leafs for this path, so create a new leaf_kinds structure
              let mut leaf_kinds = LeafKinds::new();
              leaf_kinds.insert(Leaf::new(value_type.clone()));
              self.leaf_paths.insert(path, leaf_kinds);
            }
          }
        }
        Event::Finished => todo!("schema Event::Finished"),
        Event::Error(_) => todo!("schema Event::Error"),
    }
  }
}

impl std::fmt::Display for SchemaCollector {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error>
  {
    for (p,kinds) in &self.leaf_paths {
      const WIDTH : usize = 40;
      // because otherwise 50 width is applied to each element of k

      // kinds is a Set, which doesn't really have a concept of .first()
      // so just collect the pieces as an iterator.
      let mut kfmts = kinds
        .iter()
        .map(|k| format!("{k:WIDTH$}") )
        .collect::<Vec<String>>();

      let kfmt = match kinds.len() {
        0 => String::new(),
        // no point creating another string here, and first == last
        1 => kfmts.pop().unwrap(),
        _ => format!("[{}]", kfmts.join(","))
      };

      write!(f, "{kfmt:35} {p}\n")?;
    };
    Ok(())
  }
}

impl Sender<Event<SchemaType>> for SchemaCollector {
  type SendError = String;

  // Fit in with what visitor wants
  fn send<'a>(&mut self, ev: Box<Event<SchemaType>>) -> Result<(), Self::SendError> {
    Ok(self.process_event(&ev))
  }
}

pub fn schema(jev : &mut dyn JsonEvents<String>) {
  // collect and display schema of input
  let mut collector = SchemaCollector::new();

  // translate start/end streaming events to leaf types
  let visitor = EventConverter::new();

  match visitor.value(jev, JsonPath::new(), 0, &mut collector ) {
    Ok(()) => println!("{collector}"),
    Err(err) => { eprintln!("ending event reading because {err:?}") },
  }
}

pub fn sizes(wr : &mut dyn std::io::Write) -> std::io::Result<()> {
  use std::mem::size_of;
  writeln!(wr, "jsonpath::Step {}", size_of::<crate::jsonpath::Step>())?;
  writeln!(wr, "jsonpath::JsonPath {}", size_of::<crate::jsonpath::JsonPath>())?;
  writeln!(wr, "plain::JsonEvent<String> {}", size_of::<crate::parser::JsonEvent<String>>())?;
  writeln!(wr, "parser::JsonEvent<&str> {}", size_of::<crate::parser::JsonEvent<&str>>())?;
  writeln!(wr, "sender::Event<Vec<u8>> {}", size_of::<crate::sender::Event<Vec<u8>>>())?;
  writeln!(wr, "sender::Event<&Vec<u8>> {}", size_of::<crate::sender::Event<&Vec<u8>>>())?;
  writeln!(wr, "sender::Event<u8> {}", size_of::<crate::sender::Event<u8>>())?;
  writeln!(wr, "sender::Event<&u8> {}", size_of::<crate::sender::Event<&u8>>())?;
  writeln!(wr, "schema::SchemaType {}", size_of::<crate::schema::SchemaType>())?;
  writeln!(wr, "schema::Leaf {}", size_of::<crate::schema::Leaf>())?;
  Ok(())
}
