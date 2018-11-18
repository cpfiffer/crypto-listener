use influx_db_client::{Client, Point, Points, Precision, Value};
use serde_json;
use std::collections::HashMap;

const SEPARATE_FIELDS: bool = true;

pub fn inject_influx(message: String, exchange: &'static str) {
  let client = Client::new("http://127.0.0.1:8086", exchange).set_authentication("root", "root");

  let mut point = point!(&exchange);

  if SEPARATE_FIELDS {
    let message_json: HashMap<String, serde_json::Value> = serde_json::from_str(&message).unwrap();

    for (key, value) in message_json.iter() {
      // Extract field name.
      let field_name = key.to_string();

      // Pair value type with field type.
      match value {
        serde_json::Value::Number(x) => {
          if value.is_f64() {
            point.add_field(field_name, Value::Float(x.as_f64().unwrap()));
          } else {
            point.add_field(field_name, Value::Integer(x.as_i64().unwrap()));
          }
        }
        serde_json::Value::Bool(x) => {
          point.add_field(field_name, Value::Boolean(*x));
        }
        _ => {
          point.add_field(field_name, Value::String(value.to_string()));
        }
      }
    }
  } else {
    point.add_field("message", Value::String(message));
  }

  // let point1 = Point::new("test1")
  //   .add_tag("tags", Value::String(String::from("\\\"fda")))
  //   .add_tag("number", Value::Integer(12))
  //   .add_tag("float", Value::Float(12.6))
  //   .add_field("fd", Value::String("'3'".to_string()))
  //   .add_field("quto", Value::String("\\\"fda".to_string()))
  //   .add_field("quto1", Value::String("\"fda".to_string()))
  //   .to_owned();

  let points = points!(point);

  // if Precision is None, the default is second
  // Multiple write
  let _ = client
    .write_points(points, Some(Precision::Nanoseconds), None)
    .unwrap();
}
