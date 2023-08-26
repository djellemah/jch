// This is a lot of machinery just to call a function :-\
pub struct FnSnd<T>(pub fn(T) -> ());

// This is identical to std::sync::mpsc::SendError
#[derive(Debug)]
pub struct SendError<T>(pub T);

impl<T> super::Sender<T> for FnSnd<T> {
  type SendError = SendError<T>;

  fn send(&mut self, t: T) -> Result<(), SendError<T>> {
    Ok(self.0(t))
  }
}

// use super::JsonEvents;
// use super::JsonPath;
// use super::Event;

// #[allow(dead_code)]
// pub fn values<V>(jev : &mut JsonEvents, match_path : fn(&JsonPath) -> bool)
// where V : std::fmt::Display
// {
//   // call handler with specified paths
//   let handler = FnSnd(|t : Event<V>| {
//     match t {
//       Event::Path(_depth,_path) => (),
//       // Event::Path(depth,path) => println!("path: {depth},{path}"),
//       Event::Value(p,v) => println!("jq path: [{p:#o},{v}]"),
//       Event::Finished => (),
//     }
//   });

//   use super::Handler;
//   let visitor = super::MsgPacker(match_path);
//   match visitor.value(jev, JsonPath::new(), 0, &handler ) {
//     Ok(()) => (),
//     Err(err) => { eprintln!("ending event reading {err:?}") },
//   }
// }

// #[allow(dead_code)]
// pub fn paths<DV>(jev : &mut JsonEvents)
// where
//   DV: std::fmt::Display + std::fmt::Debug, FnSnd<Event<DV>>: crate::Sender<Event<DV>>
// {
//   // call handler with specified paths
//   let handler = FnSnd(|t : Event<DV>| {
//     match t {
//       Event::Path(depth,path) => println!("{depth},{path}"),
//       Event::Value(p,v) => println!("{p} => {v}"),
//       Event::Finished => (),
//     }
//   });

//   use super::Handler;
//   let visitor = super::Plain;
//   match visitor.value::<FnSnd<Event<DV>>>(jev, JsonPath::new(), 0, &handler ) {
//     Ok(()) => (),
//     Err(err) => { eprintln!("ending event reading {err:?}") },
//   }
// }

