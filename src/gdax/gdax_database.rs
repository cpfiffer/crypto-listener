extern crate postgres;
extern crate serde_json;

use std::io;
use self::postgres::{Connection, TlsMode};

use gdax::types::*;

pub fn connect() -> Connection {
    let conn = Connection::connect("postgres://rust:fuck@localhost:5432/gdax", TlsMode::None).unwrap();

    match conn.execute("CREATE TABLE IF NOT EXISTS messages (
                message JSONB
            )", &[]) {
                Ok(_) => (),
                Err(e) => println!("Trade table creation error: {:?}", e)
            }

    return conn;
}

pub fn inject(conn: &Connection,
    val: String) -> Result<(), io::Error> {

    let prep = conn.prepare("INSERT INTO (messages) VALUES ($1)")?;

    prep.execute(&[&val])?;

    return Ok(());
}
