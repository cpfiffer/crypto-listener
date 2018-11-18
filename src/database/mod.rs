extern crate postgres;
extern crate serde_json;

use self::postgres::{Connection, TlsMode};
use std::io;

use super::gemini::{Change, Trade, Update};

pub mod influx;

const TRADE_INJECT: &'static str = "
    INSERT INTO trades (
    timestampms,
    socket_sequence,
    eventId,
    tid,
    price,
    amount,
    makerSide,
    pair)
    VALUES (
        $1, $2, $3, $4, $5, $6, $7, $8
    )";
const CHANGE_INJECT: &'static str = "
    INSERT INTO changes (
        timestampms,
        socket_sequence,
        eventId,
        reason,
        price,
        delta,
        remaining,
        side,
        pair)
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9
        )";

pub fn connect() -> Connection {
    let conn =
        Connection::connect("postgres://rust:fuck@localhost:5432/gemini", TlsMode::None).unwrap();

    match conn.execute(
        "CREATE TABLE IF NOT EXISTS trades (
                timestampms    bigint,
                socket_sequence oid,
                eventId oid,
                tid oid,
                price text,
                amount text,
                makerSide text,
                pair text
            )",
        &[],
    ) {
        Ok(_) => (),
        Err(e) => println!("Trade table creation error: {:?}", e),
    }

    match conn.execute(
        "CREATE TABLE IF NOT EXISTS changes (
              timestampms    bigint,
              socket_sequence oid,
              eventId oid,
              reason text,
              price text,
              delta text,
              remaining text,
              side text,
              pair text
          )",
        &[],
    ) {
        Ok(_) => (),
        Err(e) => print!("Change table creation injection error: {:?}", e),
    }

    return conn;
}

pub fn inject_gemini(
    conn: &Connection,
    val: Update,
    trades: Vec<Trade>,
    changes: Vec<Change>,
    pair: &'static str,
) -> Result<(), io::Error> {
    // Inject trades.
    let prep = conn.prepare(TRADE_INJECT)?;
    for t in trades {
        match prep.execute(&[
            &val.timestampms,
            &val.socket_sequence,
            &val.eventId,
            &t.tid,
            &t.price,
            &t.amount,
            &t.makerSide,
            &pair,
        ]) {
            Ok(_) => (),
            Err(e) => println!("Trade injection error: {:?}", e),
        };
    }

    // Inject changes.
    let prep = conn.prepare(CHANGE_INJECT)?;
    for c in changes {
        match prep.execute(&[
            &val.timestampms,
            &val.socket_sequence,
            &val.eventId,
            &c.reason,
            &c.price,
            &c.delta,
            &c.remaining,
            &c.side,
            &pair,
        ]) {
            Ok(_) => (),
            Err(e) => print!("Change injection error: {:?}", e),
        };
    }

    return Ok(());
}
