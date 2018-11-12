extern crate postgres;
extern crate serde_json;

use self::postgres::{Connection, TlsMode};
use serde_json::Value;
use std::io;

// use gdax::types::*;

pub fn connect() -> Connection {
    let conn =
        Connection::connect("postgres://rust:fuck@localhost:5432/gdax", TlsMode::None).unwrap();

    match conn.execute(
        "CREATE TABLE IF NOT EXISTS messages (
                message JSONB
            )",
        &[],
    ) {
        Ok(_) => (),
        Err(e) => println!("Trade table creation error: {:?}", e),
    }

    return conn;
}

pub fn inject(conn: &Connection, val: String) -> Result<(), io::Error> {
    let val2: Value = serde_json::from_str(&val).unwrap();
    let prep = conn.prepare("INSERT INTO messages (message) VALUES ($1)")?;
    prep.execute(&[&val2])?;
    return Ok(());
}
