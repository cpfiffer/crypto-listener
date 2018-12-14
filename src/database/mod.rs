extern crate postgres;
extern crate serde_json;

use self::postgres::{Connection, TlsMode};

pub fn connect(exchange: &str, password: String) -> Connection {
    let conn = Connection::connect(
        format!("postgres://rust:{}@localhost:5432/{}", password, exchange),
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

pub fn inject_json(conn: &Connection, message: String) {
    let js: serde_json::Value = serde_json::from_str(&message).unwrap();
    let r = conn.execute("INSERT INTO raw_json VALUES (NOW(), $1)", &[&js]);

    match r {
        Ok(_) => {}
        Err(x) => println!("Inserting json error: {:?} Message: {:?}", x, &message),
    }
}

pub fn inject_log(conn: &Connection, message: String) {
    let r = conn.execute("INSERT INTO log VALUES (NOW(), $1)", &[&message]);

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
    let r = conn.execute("INSERT INTO log VALUES (NOW(), $1, $2)", &[&message, &js]);

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
