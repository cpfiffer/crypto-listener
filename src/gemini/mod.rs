extern crate postgres;
extern crate serde;
extern crate serde_json;
extern crate websocket;

use crate::database;
use crate::errors::CryptoError;
use crate::threadpack::*;

use hyper;
use serde_json::Value;
use std::io::Read;
use std::sync::mpsc::Receiver;
use std::thread;
use websocket::client::sync::Client;
use websocket::client::ClientBuilder;
use websocket::stream::sync::NetworkStream;
use websocket::Message;
use websocket::OwnedMessage;

// Connection strings.
const CONNECTION: &'static str = "wss://api.gemini.com/v1/marketdata/";
const THIS_EXCHANGE: &'static str = "gemini";
const SYMBOL_REQUEST: &'static str = "https://api.gemini.com/v1/symbols";

pub fn start_gemini(
    rvx: &mut bus::Bus<ThreadMessages>,
    live: bool,
    password: String,
) -> (Vec<thread::JoinHandle<()>>, Vec<Receiver<ThreadMessages>>) {
    // let connection_targets:
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();
    let mut receivers: Vec<Receiver<ThreadMessages>> = Vec::new();

    let prods = get_products();

    let (tpack, receiver) = ThreadPack::new(rvx.add_rx(), THIS_EXCHANGE);
    threads.push(spin_thread(tpack, live, password, prods));
    receivers.push(receiver);

    return (threads, receivers);
}

pub fn spin_thread(
    mut tpack: ThreadPack,
    live: bool,
    password: String,
    products: Vec<String>,
) -> thread::JoinHandle<()> {
    // Run through all our clients.
    return thread::spawn(move || {
        if live {
            loop {
                // Allocate a client vector.
                let mut clients: Vec<Client<Box<dyn NetworkStream + Send>>> = Vec::new();

                // Connect to the database.
                let conn = database::connect(THIS_EXCHANGE, password.clone());

                // Spin up some clients.
                for pair in &products {
                    // let target = [CONNECTION, pair].join("");
                    let target = format!("{}{}", CONNECTION, pair.replace("-", ""));
                    tpack.message(format!("Connecting to {}...", &target));

                    // Wait for messages back.
                    if live {
                        let client = ClientBuilder::new(&target).unwrap().connect(None).unwrap();
                        tpack.message(format!("Successfully connected to {}!", &target));
                        clients.push(client);
                    }
                }
                let (t, result) = iterate_clients(tpack, clients, conn);
                tpack = t;

                match result {
                    Ok(_) => {}
                    Err(CryptoError::Nothing) => {}
                    Err(CryptoError::Restartable) => {
                        tpack.message(format!(
                            "{} received a restartable error, restarting...",
                            THIS_EXCHANGE
                        ));
                    }
                    Err(CryptoError::NonRestartable) => {
                        tpack.message(format!(
                            "{} received a nonrestartable error, closing...",
                            THIS_EXCHANGE
                        ));
                        tpack.notify_abort();

                        break;
                    }
                }
            }
        }
        return ();
    });
}

pub fn iterate_clients(
    mut tpack: ThreadPack,
    mut clients: Vec<Client<Box<dyn NetworkStream + Send>>>,
    conn: postgres::Connection,
) -> (ThreadPack, Result<(), CryptoError>) {
    // Check our messages.
    let mut do_close = false;

    // Define error state.
    let mut error_state = CryptoError::Nothing;

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
                    error_state = CryptoError::Restartable;
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
                    error_state = CryptoError::Restartable;
                }
            }

            // Check if we've been told to close.
            if tpack.check_close() {
                println!("{} received close message.", tpack.exchange);
                do_close = true;
                error_state = CryptoError::NonRestartable;
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

    match error_state {
        CryptoError::Nothing => return (tpack, Ok(())),
        _ => return (tpack, Err(error_state)),
    }
}

fn get_products() -> Vec<String> {
    let body = reqwest::get(SYMBOL_REQUEST).unwrap().text().unwrap();
    let body: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    let mut products = vec![];

    for item in body.iter() {
        let id = match &item {
            serde_json::Value::String(x) => x.to_owned(),
            _ => {
                info!("Error fetching products on {}, {:?}", THIS_EXCHANGE, &body);
                String::new()
            }
        };

        products.push(id);
    }

    return products;
}
