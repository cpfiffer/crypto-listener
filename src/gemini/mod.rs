extern crate postgres;
extern crate websocket;

use crate::database;
use crate::errors::CryptoError;
use crate::threadpack::*;
use crate::wspack::WSPack;

use serde_json;
use serde_json::Value;
use serde_json::Map;
use serde_json::json;
use hyper;
use std::sync::mpsc::Receiver;
use std::thread;
use websocket::client::sync::Client;
use websocket::stream::sync::NetworkStream;
use websocket::Message;
use websocket::OwnedMessage;

// Connection strings.
const CONNECTION: &'static str = "wss://api.gemini.com/v1/marketdata/";
const THIS_EXCHANGE: &'static str = "gemini";
const SYMBOL_REQUEST: &'static str = "https://api.gemini.com/v1/symbols";

pub fn extra_handling(incoming: String,) -> String {
let mut p:Map<String, Value> = serde_json::from_str(&incoming).unwrap();
                p.entry("pair").or_insert(json!(pair));
                let p = serde_json::to_value(p).unwrap();
                let mess = serde_json::to_string_pretty(&p).unwrap();
                println!("{}", &mess);
                return mess;
}

pub fn start_gemini(
    rvx: &mut bus::Bus<ThreadMessages>,
    live: bool,
) -> (Vec<WSPack>, Vec<Receiver<ThreadMessages>>) {
    // Allocate return vectors.
    // let mut clients: Vec<WSPack> = Vec::new();
    // let mut receivers: Vec<Receiver<ThreadMessages>> = Vec::new();

    // Get products.
    let prods = get_products();

    // Generate clients.
    let (clients, receivers) = spin_clients(live, prods, rvx);

    return (clients, receivers);
}

pub fn spin_clients(
    live: bool,
    products: Vec<String>,
    rvx: &mut bus::Bus<ThreadMessages>,
) -> (Vec<WSPack>, Vec<Receiver<ThreadMessages>>) {
    // Allocate return vectors.
    let mut clients: Vec<WSPack> = Vec::new();
    let mut receivers: Vec<Receiver<ThreadMessages>> = Vec::new();

    // Run through all our clients.
    if live {
        // Spin up some clients.
        for pair in &products {
            // Get URI.
            let target = format!("{}{}", CONNECTION, &pair);

            // Create closure to add pairs to each json message.
            let handle_func = |incoming:String| {
                
            };

            // Make a client.
            let (new_client, rx) = WSPack::new(target, THIS_EXCHANGE, rvx.add_rx(), handle_func);

            // Stash the new client and receiver.
            clients.push(new_client);
            receivers.push(rx);
        }
    }

    // Return.
    return (clients, receivers);
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
