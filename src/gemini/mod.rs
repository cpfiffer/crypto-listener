extern crate hyper;
extern crate postgres;
extern crate serde;
extern crate serde_json;
extern crate websocket;

use crate::database;
use crate::threadpack::*;

use hyper::header::Headers;
use std::sync::mpsc::Receiver;
use std::thread;
use websocket::client::sync::Client;
use websocket::client::ClientBuilder;
use websocket::stream::sync::NetworkStream;
use websocket::Message;
use websocket::OwnedMessage;

// Connection strings.
const CONNECTION: &'static str = "wss://api.gemini.com/v1/marketdata/";
const PAIRS: &'static [&'static str] =
    &["BTCUSD", "ETHUSD", "ETHBTC", "ZECUSD", "ZECBTC", "ZECETH"];
const THIS_EXCHANGE: &'static str = "gemini";

pub fn start_gemini(
    rvx: &mut bus::Bus<ThreadMessages>,
    live: bool,
    password: String,
) -> (Vec<thread::JoinHandle<()>>, Vec<Receiver<ThreadMessages>>) {
    // let connection_targets:
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();
    let mut receivers: Vec<Receiver<ThreadMessages>> = Vec::new();
    let conn = database::connect(THIS_EXCHANGE, password);

    let (tpack, receiver) = ThreadPack::new(rvx.add_rx(), THIS_EXCHANGE);
    threads.push(spin_thread(tpack, live, conn));
    receivers.push(receiver);

    return (threads, receivers);
}

pub fn spin_thread(
    tpack: ThreadPack,
    live: bool,
    conn: postgres::Connection,
) -> thread::JoinHandle<()> {
    // Allocate a client vector.
    let mut clients: Vec<Client<Box<dyn NetworkStream + Send>>> = Vec::new();

    // Spin up some clients.
    for pair in PAIRS {
        let target = [CONNECTION, pair].join("");
        println!("Connecting to {}...", &target);

        // Wait for messages back.
        if live {
            let client = ClientBuilder::new(&target).unwrap().connect(None).unwrap();
            println!("Successfully connected to {}!", &target);
            clients.push(client);
        }
    }

    // Run through all our clients.
    return thread::spawn(move || {
        if live {
            iterate_clients(tpack, clients, conn);
        }
        return ();
    });
}

pub fn iterate_clients(
    mut tpack: ThreadPack,
    mut clients: Vec<Client<Box<dyn NetworkStream + Send>>>,
    conn: postgres::Connection,
) {
    // Check our messages.
    let mut do_close = false;
    while !do_close {
        for i in clients.iter_mut() {
            let m = i.recv_message();

            match m {
                Ok(OwnedMessage::Close(_)) => {
                    do_close = true;
                    database::inject_log(
                        &conn,
                        format!("{} received termination from server.", tpack.exchange),
                    );
                }
                Ok(OwnedMessage::Binary(_)) => {}
                Ok(OwnedMessage::Ping(_)) => {}
                Ok(OwnedMessage::Pong(_)) => {}
                Ok(OwnedMessage::Text(x)) => {
                    database::inject_json(&conn, x);
                }
                Err(x) => {
                    tpack.message(format!(
                        "Error in {} receiving messages: {:?}",
                        tpack.exchange, x
                    ));
                    do_close = true;
                }
            }

            // Check if we've been told to close.
            if tpack.check_close() {
                println!("{} received close message.", tpack.exchange);
                do_close = true;
                break;
            }
        }
    }

    // Wind the thread down.
    for i in clients.iter_mut() {
        let m_close = i.send_message(&Message::close());
        match m_close {
            Ok(_) => {}
            Err(x) => {
                let mess = format!(
                    "Error closing client for thread {:?}, message: {}",
                    thread::current().id(),
                    x
                );

                tpack.message(mess.to_string());
            }
        }
        database::notify_terminate(&conn, THIS_EXCHANGE);
        tpack.notify_closed();
    }

    ()
}

pub fn gemini_headers() -> Headers {
    let mut headers = Headers::new();

    headers.append_raw("X-GEMINI-APIKEY", b"fuck".to_vec());
    headers.append_raw("X-GEMINI-PAYLOAD", b"fuck".to_vec());
    headers.append_raw("X-GEMINI-SIGNATURE", b"fuck".to_vec());

    return headers;
}
