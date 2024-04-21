/*!
This traverses/handles the incoming json events from the streaming parser.
*/
use crate::parser::JsonEvents;
use crate::sender::Sender;
use crate::sender::Event;
use crate::jsonpath::*;

use json_event_parser::JsonEvent;

/**
The Handler trait.

A place to hang `match_path` and `maybe_send_value` without
threading those functions through the JsonEvent handlers.

Effectively it's a
visitor with accept = match_path and visit = maybe_send_value
*/
pub trait Handler {
  /// value contained by Event
  // Lifetime bound is so that events are allowed the shortest lifetime possible,
  // hence the where clauses and higher-ranked for declarations in the below trait methods.
  type V<'l> where Self : 'l;

  // TODO this is optional?
  fn match_path(&self, path : &JsonPath) -> bool;

  /// This will be called for each leaf value, along with its path.
  fn maybe_send_value<'a, Snd>(&self, path : &JsonPath, ev : &JsonEvent, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  // the `for` is critical here because 'x must have a longer lifetime than 'a but a shorter lifetime than 'l
  where
    Snd : for <'x> Sender<Event<Self::V<'x>>>,
    for <'x> <Self as Handler>::V<'x> : std::fmt::Debug
  ;

  /// Handle all arrays.
  /// values will be emitted via maybe_send_value
  /// nested arrays are recursive
  /// objects are sent to object(...)
  // TODO why is depth here? It's duplicated in the parents path.
  fn array<'a, Snd>(&self, jevs : &mut JsonEvents, parents : JsonPath, depth : u64, tx : &mut Snd )
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  where
    Snd : for <'x> Sender<Event<Self::V<'x>>>,
    for <'x> <Self as Handler>::V<'x>: std::fmt::Debug
  {
    let mut index = 0;
    let mut buf : Vec<u8> = vec![];
    while let Some(ref ev) = jevs.next_buf(&mut buf) {
      // NOTE rpds persistent vector
      let loop_parents = parents.push_back(index.into());
      use JsonEvent::*;
      let res = match ev {
        // ok we have a leaf, so match path then send value
        String(_) | Number(_)  | Boolean(_) | Null => self.maybe_send_value(&parents, &ev, tx),

        StartArray => self.array(jevs, loop_parents, depth+1, tx),
        EndArray => return Ok(()), // do not send path, this is +1 past the end of the array

        // ObjectKey(key) => find_path(jevs, loop_parents.push_front(key.into()), depth+1, tx),
        StartObject => self.object(jevs, loop_parents, depth+1, tx),
        ObjectKey(_) => panic!("should never receive ObjectKey {parents}"),
        EndObject => panic!("should never receive EndObject {parents}"),

        Eof => tx.send(Box::new(Event::Finished)),
      };
      match res {
          Ok(()) => (),
          err => return err,
      }
      index += 1;
    }
    Ok(())
  }

  /// handle objects.
  fn object<'a, Snd>(&self, jevs : &mut JsonEvents, parents : JsonPath, depth : u64, tx : &mut Snd )
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  where
    Snd : for <'x> Sender<Event<Self::V<'x>>>,
    for <'x> <Self as Handler>::V<'x> : std::fmt::Debug
  {
    let mut buf : Vec<u8> = vec![];
    while let Some(ref ev) = jevs.next_buf(&mut buf) {
      use JsonEvent::*;
      let res = match &ev {
        // ok we have a leaf, so emit the value and path.
        // no need to shunt this through value(...)
        String(_) | Number(_)  | Boolean(_) | Null => self.maybe_send_value(&parents, &ev, tx),

        StartArray => self.array(jevs, parents.clone(), depth+1, tx),
        EndArray => panic!("should never receive EndArray {parents}"),

        StartObject => self.value(jevs, parents.clone(), depth+1, tx),
        ObjectKey(key) => self.value(jevs, parents.push_back((*key).into()), depth+1, tx),
        EndObject => return Ok(()),

        // fin
        Eof => tx.send(Box::new(Event::Finished)),
      };
      match res {
          Ok(()) => (),
          err => return err,
      }
    }
    Ok(())
  }

  /// Handle String Number Boolean Null (ie non-recursive)
  fn value<'a,Snd>(&self, jevs : &mut JsonEvents, parents : JsonPath, depth : u64, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  where
    Snd : for <'x> Sender<Event<Self::V<'x>>>,
    for <'x> <Self as Handler>::V<'x> : std::fmt::Debug
  {
    let mut buf : Vec<u8> = vec![];
    // json has exactly one top-level object
    if let Some(ref ev) = jevs.next_buf(&mut buf) {
      use JsonEvent::*;
      match ev {
        // ok we have a leaf, so emit the value and path
        String(_) | Number(_)  | Boolean(_) | Null => self.maybe_send_value(&parents, &ev, tx),

        StartArray => self.array(jevs, parents, depth+1, tx),
        EndArray => panic!("should never receive EndArray {parents}"),

        StartObject => self.object(jevs, parents, depth+1, tx),
        ObjectKey(_) => panic!("should never receive ObjectKey {parents}"),
        EndObject => panic!("should never receive EndObject {parents}"),

        // fin
        Eof => tx.send(Box::new(Event::Finished)),
      }
    } else {
      tx.send(Box::new(Event::Finished))
    }
  }
}
