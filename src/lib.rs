/*!
Parse really large json files, fast. Using a streaming parser so memory usage is limited.

For example:

- the schema for a 26Mb file is calculated in about 750ms using about 4Mb of RAM.

- the schema for a 432Mb file is calculated in about 20s using about 4Mb of RAM.

Currently only outputs the schema of a json file, in a non-standard format.

Is designed in a modular way so you can use it as a base for filtering json. Like `jq`.

Might grow some kind of path-filtering languages, like jsonpath or xpath.
*/

// parser and traits
pub mod parser;
pub mod jsonpath;
pub mod sendpath;
pub mod sender;
pub mod handler;

// handlers and sender implementations
pub mod plain;
pub mod shredder;
pub mod schema;
pub mod valuer;
pub mod channel;
pub mod fn_snd;

/// The idea here was something like ruby's ARGF, ie stdin and then all command line args that are files.
/// But currently it only handles either stdin or a single file.
pub fn make_readable<S>(maybe_readable_args : &[S]) -> Box<dyn std::io::BufRead>
where S : AsRef<str> + std::convert::AsRef<std::path::Path> + std::fmt::Debug
{
  // use std::io::Read;
  match &maybe_readable_args[..] {
    [] => Box::new(std::io::stdin().lock()),
    [arg_fn] => {
      let file = std::fs::File::open(arg_fn).expect("cannot open file {arg_fn}");
      Box::new(std::io::BufReader::new(file))
    }
    _ => panic!("too many args {maybe_readable_args:?}")
  }
}
