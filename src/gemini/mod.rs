#![allow(non_snake_case)]

extern crate hyper;
extern crate postgres;
extern crate serde;
extern crate serde_json;
extern crate websocket;

use super::threadpack::*;
use hyper::header::Headers;
use influx;
use influx::*;
use std::sync::mpsc::Receiver;
use std::thread;
use websocket::client::ClientBuilder;
use websocket::{Message, OwnedMessage};

// Connection strings.
const CONNECTION: &'static str = "wss://api.gemini.com/v1/marketdata/";
const PAIRS: &'static [&'static str] =
    &["BTCUSD", "ETHUSD", "ETHBTC", "ZECUSD", "ZECBTC", "ZECETH"];
const THIS_EXCHANGE: &'static str = "gemini";

pub fn start_gemini(rvx: &mut bus::Bus<ThreadMessages>, live: bool) -> 
(Vec<thread::JoinHandle<()>>, Vec<Receiver<ThreadMessages>>) {
    // let connection_targets:
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();
    let mut receivers: Vec<Receiver<ThreadMessages>> = Vec::new();

    for pair in PAIRS {
        let (mut tpack, mut receiver) = ThreadPack::new(rvx.add_rx());
        threads.push(spin_thread(pair, tpack, live));
        receivers.push(receiver)
    }

    return (threads, receivers);
}

pub fn spin_thread(
    pair: &'static str,
    mut tpack: ThreadPack,
    live: bool,
) -> thread::JoinHandle<()> {
    // Spin up some threads and return them.
    let target = [CONNECTION, pair].join("");
    println!("Connecting to {}...", &target);

    // Wait for messages back.
    let listen_thread = thread::spawn(move || {
        if live {
            let mut client = ClientBuilder::new(&target).unwrap().connect(None).unwrap();

            println!("Successfully connected to {}!", &target);

            // Check our messages.
            for message in client.incoming_messages() {
                if let Ok(OwnedMessage::Text(x)) = message {
                    inject_influx(x, THIS_EXCHANGE);
                } else {
                    tpack.message(format!(
                        "Error in {} receiving messages: {:?}",
                        THIS_EXCHANGE, message
                    ));
                    break;
                };

                // Check if we've been told to close.
                if tpack.check_close() {
                    break;
                }
            }

            // Wind the thread down.
            let m_close = client.send_message(&Message::close());
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
            influx::influx_termination(THIS_EXCHANGE);
            tpack.notify_closed();
        } else {
            tpack.message("Thread {:?} closing, not live.".to_string());
            tpack.notify_closed();
            ()
        }
    });

    return listen_thread;
}

pub fn gemini_headers() -> Headers {
    let mut headers = Headers::new();

    headers.append_raw("X-GEMINI-APIKEY", b"fuck".to_vec());
    headers.append_raw("X-GEMINI-PAYLOAD", b"fuck".to_vec());
    headers.append_raw("X-GEMINI-SIGNATURE", b"fuck".to_vec());

    return headers;
}
