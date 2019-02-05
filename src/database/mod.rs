extern crate postgres;
extern crate serde_json;

use self::postgres::{Connection, TlsMode};

use std::collections::HashMap;
use serde_json::map::Map;

// Predefined static SQL queries.
const LOG_INJECT: &'static str = "INSERT INTO log VALUES (NOW(), $1)";
const LOG_INJECT_ADDITIONAL: &'static str = "INSERT INTO log VALUES (NOW(), $1, $2)";
const JSON_INJECT: &'static str = "INSERT INTO raw_json VALUES (NOW(), $1)";

pub fn connect(exchange: &str) -> Connection {
    let conn = Connection::connect(
        format!("postgres://rust:{}@localhost:5432/{}", "fuck", exchange),
        TlsMode::None,
    )
    .unwrap();

    match conn.execute(
        "CREATE TABLE IF NOT EXISTS raw_json (
        time        TIMESTAMPTZ       NOT NULL,
        message    JSONB  NULL)",
        &[],
    ) {
        Ok(_) => (),
        Err(e) => println!("raw_json creation error: {:?}", e),
    }

    match conn.execute(
        "CREATE TABLE IF NOT EXISTS log (
        time        TIMESTAMPTZ       NOT NULL,
        message    TEXT  NULL,
        additional JSONB NULL)",
        &[],
    ) {
        Ok(_) => (),
        Err(e) => print!("log creation error: {:?}", e),
    }

    inject_log(&conn, format!("{} connected to database", exchange));

    return conn;
}

pub fn inject_json(conn: &Connection, message: String, extra: &HashMap<String, String>) {
    let mut js: Map<String, serde_json::Value> = serde_json::from_str(&message).unwrap();
    for (key, value) in &*extra {
        js.insert(key.to_string(), serde_json::value::Value::String(value.to_string()));
    }
    let jsv: serde_json::Value = serde_json::to_value(js).unwrap();

    let r = conn.execute(JSON_INJECT, &[&jsv]);

    match r {
        Ok(_) => {}
        Err(x) => println!("Inserting json error: {:?} Message: {:?}", x, &message),
    }
}

pub fn inject_log(conn: &Connection, message: String) {
    let r = conn.execute(LOG_INJECT, &[&message]);

    match r {
        Ok(_) => {}
        Err(x) => println!("Inserting log error: {:?}", x),
    }
}

pub fn notify_terminate(conn: &Connection, exchange: &'static str) {
    inject_log(conn, format!("{} terminated", exchange));
}

pub fn inject_log_additional(conn: &Connection, message: String, additional: String) {
    let js: serde_json::Value = serde_json::from_str(&additional).unwrap();
    let r = conn.execute(LOG_INJECT_ADDITIONAL, &[&message, &js]);

    match r {
        Ok(_) => {}
        Err(x) => println!("Inserting log_additional error: {:?}", x),
    }
}

// fn postgres_string(name: &String, chunk: &Value) -> String {
//     match chunk {
//         Value::Null => format!("{} NULL", &name),
//         Value::Bool(_) => format!("{} BOOL", &name),
//         Value::Number(x) => {
//             if x.is_f64() {
//                 return format!("{} DOUBLE PRECISION", &name);
//             } else if x.is_i64() {
//                 return format!("{} BIGINT", &name);
//             } else if x.is_u64() {
//                 return format!("{} OID", &name);
//             }

//             return format!("{} REAL", &name);
//         }
//         Value::String(_) => format!("{} TEXT", &name),
//         Value::Array(x) => format!("{} ARRAY", &name),
//         Value::Object(_) => return "".to_string(),
//     }
// }

// enum Value {
//     Null,
//     Bool(bool),
//     Number(Number),
//     String(String),
//     Array(Vec<Value>),
//     Object(Map<String, Value>),
// }
