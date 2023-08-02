
#[allow(dead_code)]
fn keys_only(pit : &mut json_stream::parse::ParseObject) {
  while let Some(parse_result) = pit.next() {
    match parse_result {
      Ok(mut kv) => {
        match kv.key().read_owned() {
          Ok(key) => {
            println!("key: {key:?}");
            // drop(kv); should be called automatically anyway
          }
          Err(err) => eprintln!("read key failed {err:?}"),
        }
      }
      Err(v) => eprintln!("read kv failed {v:?}"),
    };
    println!("read next key")
  }
}


#[allow(dead_code)]
fn old_main() {
  let istream = make_readable();
  let mut top_stream = json_stream::parse::Parser::new(istream);
  while let Some(pobject) = top_stream.next() {
    match pobject {
      Ok(mut parse_object) => {
        use json_stream::parse::Json::*;
        match &mut parse_object {
          Null => println!("Null"),
          Bool(_) => println!("Bool"),
          Number(_) => println!("Number"),
          String(_) => println!("String"),
          Array(_parse_array) => {println!("Array")},
          Object(parse_object) => {
            // println!("Object {:?}", parse_object);
            keys_only(parse_object);
            drop(parse_object)
          }
        }
      }
      Err(err) => eprintln!("{err:?}"),
    }
  }
}
