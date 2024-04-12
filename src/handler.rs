use crate::parser::JsonEvents;
use crate::sendpath::Sender;
use crate::sendpath::Event;
use crate::jsonpath::*;

// This traverses/handles the incoming json stream events.
//
// Really just becomes a place to hang match_path and maybe_send_value without
// threading those functions through the JsonEvent handlers. Effectively it's a
// visitor with accept = match_path and visit = maybe_send_value
pub trait Handler {

  // value contained by Event
  // Lifetime bound is so that events are allowed the shortest lifetime possible,
  // hence the where clauses and higher-ranked for declarations in the below trait methods.
  type V<'l> where Self : 'l;

  fn match_path(&self, path : &JsonPath) -> bool;

  // default implementation that does nothing and returns OK
  fn maybe_send_value<'a, Snd>(&self, path : &JsonPath, ev : &json_event_parser::JsonEvent, tx : &mut Snd)
  -> Result<(),<Snd as crate::sendpath::Sender<crate::sendpath::Event<<Self as Handler>::V<'_>>>>::SendError>
  // the `for` is critical here because 'x must have a longer lifetime than 'a but a shorter lifetime than 'l
  where Snd : for <'x> crate::sendpath::Sender<crate::sendpath::Event<Self::V<'x>>>
  ;

  fn array<'a, Snd>(&self, jev : &mut JsonEvents, parents : JsonPath, depth : u64, tx : &mut Snd )
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  where Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
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

        Eof => tx.send(&Event::Finished),
      };
      match res {
          Ok(()) => (),
          err => return err,
      }
      index += 1;
    }
    Ok(())
  }

  fn object<'a, Snd>(&self, jev : &mut JsonEvents, parents : JsonPath, depth : u64, tx : &mut Snd )
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  where Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
    let mut buf : Vec<u8> = vec![];
    while let Some(ev) = jev.next_buf(&mut buf) {
      use json_event_parser::JsonEvent::*;
      let res = match ev {
        // ok we have a leaf, so emit the value and path
        String(_) | Number(_)  | Boolean(_) | Null => self.maybe_send_value(&parents, &ev, tx),

        StartArray => self.array(jev, parents.clone(), depth+1, tx),
        EndArray => panic!("should never receive EndArray {parents}"),

        StartObject => self.value(jev, parents.clone(), depth+1, tx),
        ObjectKey(key) => self.value(jev, parents.push_back(key.into()), depth+1, tx),
        EndObject => return Ok(()),

        // fin
        Eof => tx.send(&Event::Finished),
      };
      match res {
          Ok(()) => (),
          err => return err,
      }
    }
    Ok(())
  }

  fn value<'a,Snd>(&self, jev : &mut JsonEvents, parents : JsonPath, depth : u64, tx : &mut Snd)
  -> Result<(),<Snd as Sender<Event<<Self as Handler>::V<'_>>>>::SendError>
  where Snd : for <'x> Sender<Event<Self::V<'x>>>
  {
    let mut buf : Vec<u8> = vec![];
    // json has exactly one top-level object
    if let Some(ev) = jev.next_buf(&mut buf) {
      use json_event_parser::JsonEvent::*;
      match ev {
        // ok we have a leaf, so emit the value and path
        String(_) | Number(_)  | Boolean(_) | Null => self.maybe_send_value(&parents, &ev, tx),

        StartArray => self.array(jev, parents, depth+1, tx),
        EndArray => panic!("should never receive EndArray {parents}"),

        StartObject => self.object(jev, parents, depth+1, tx),
        ObjectKey(_) => panic!("should never receive ObjectKey {parents}"),
        EndObject => panic!("should never receive EndObject {parents}"),

        // fin
        Eof => tx.send(&Event::Finished),
      }
    } else {
      tx.send(&Event::Finished)
    }
  }
}
