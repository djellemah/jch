#[allow(dead_code)]
fn append_step(steps : &Vec<Step>, last : Step) -> Vec<Step> {
  let mut more_steps = steps.clone();
  more_steps.push(last);
  more_steps
}

fn make_indent(parents : &Parents) -> String {
  let mut indent = String::new();
  for _ in parents { indent.push(' ') };
  indent
}

fn collect_keys(jev : &mut JsonEvents, parents : &Parents) {
  let mut map : std::collections::BTreeMap<String, serde_json::Value> = std::collections::BTreeMap::new();

  // eurgh. This is a rather unpleasant pattern
  let mut buf : Vec<u8> = vec![];
  if let Some(ev) = jev.next_buf(&mut buf) {
    match ev {
      JsonEvent::ObjectKey(key) => {
        map.insert(key.to_string(), collect_value(jev, key, &parents));
        collect_keys(jev, parents);
      }
      JsonEvent::Null => todo!(),
      JsonEvent::StartArray => todo!(),
      JsonEvent::EndArray => todo!(),
      JsonEvent::StartObject => todo!(),
      JsonEvent::EndObject => todo!(),
      other => panic!("unhandled {other:?}"),
    }
  }
}

fn collect_value(jev : &mut JsonEvents, key : &str, parents : &Parents) -> serde_json::Value {
  let indent = make_indent(parents);
  if let Some(ev) = jev.next() {
    match ev {
      JsonEvent::String(val) => {
        println!("{indent}{key}: {val}");
        serde_json::Value::String(val.to_string())
      }
      JsonEvent::Number(val) => {
        println!("{indent}{key}");
        serde_json::Value::String(val.to_string())
        // serde_json::Value::Number(val.parse::<i64>().unwrap_or(serde_json::Value::Null))
      }
      _ => serde_json::Value::Null,
    }
  } else {
    serde_json::Value::Null
  }
}

#[allow(dead_code)]
fn display_keys(jev : &mut JsonEvents, parents : &Parents) {
  let mut indent = String::new();
  for _ in parents { indent.push(' ') };
  let mut map : std::collections::BTreeMap<String, Option<serde_json::Value>> = std::collections::BTreeMap::new();

  let mut buf : Vec<u8> = vec![];
  while let Some(ev) = jev.next_buf(&mut buf) {
    match ev {
      JsonEvent::StartObject => {
        println!("---");
        // display_keys(jev, &parents)
        collect_keys(jev, &parents)
      }
      JsonEvent::EndObject => return,
      JsonEvent::ObjectKey(key) => {
        println!("{indent}{key}");
        map.insert(key.to_string(), None);
      }
      JsonEvent::Eof => panic!("unexpected eof"),
      _ => (),
    }
  }
}
