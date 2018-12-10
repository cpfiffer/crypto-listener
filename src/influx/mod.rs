use influx_db_client::{Client, Point, Points, Precision, Value};
use serde_json;
use std::collections::HashMap;

extern crate serde;
extern crate serde_derive;

const SEPARATE_FIELDS: bool = true;

pub fn inject_influx(message: String, database: &'static str) {
  let client = Client::new("http://127.0.0.1:8086", database).set_authentication("root", "root");

  let mut point = point!(&database);

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

  let points = points!(point);

  // if Precision is None, the default is second
  // Multiple write
  let _ = client
    .write_points(points, Some(Precision::Nanoseconds), None)
    .unwrap();
}

#[derive(Serialize, Deserialize)]
pub struct InfluxMessage {
  message_type: String,
  message: String,
}

fn create_message(message_type: String, message: String) -> String {
  let message = serde_json::to_string(&InfluxMessage {
    message_type,
    message,
  })
  .unwrap();

  return message;
}

pub fn influx_termination(exchange: &'static str) {
  influx_message("thread_closure".to_string(), exchange.to_string(), exchange);
}

pub fn influx_message(message_type: String, message: String, database: &'static str) {
  let message = create_message(message_type, message);
  let _ = inject_influx(message, database);
}
