extern crate hyper;
extern crate hyper_native_tls;
extern crate postgres;
#[allow(non_snake_case)]
// #[feature(assoc_unix_epoch)]
extern crate serde;
extern crate serde_json;
extern crate websocket;

use self::hyper_native_tls::NativeTlsClient;
use super::threadpack::*;
use crate::database;
use hyper::header::{Headers, UserAgent};
use hyper::net::HttpsConnector;
use hyper::Client;
use serde_json::Value;
use std::io::Read;
use std::sync::mpsc::Receiver;
use std::thread;
use websocket::client::ClientBuilder;
use websocket::sync::stream::NetworkStream;
use websocket::{Message, OwnedMessage};

// pub mod gdax_database;
pub mod types;

use self::types::*;

const CONNECTION: &'static str = "wss://ws-feed.gdax.com";
const THIS_EXCHANGE: &'static str = "gdax";

pub fn start_gdax(
    rvx: bus::BusReader<ThreadMessages>,
    live: bool,
    password: String,
) -> (Vec<thread::JoinHandle<()>>, Vec<Receiver<ThreadMessages>>) {
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();
    let mut receivers: Vec<Receiver<ThreadMessages>> = Vec::new();
    let conn = database::connect(THIS_EXCHANGE, password);

    let (handle, receiver) = spin_thread(rvx, live, conn);
    threads.push(handle);
    receivers.push(receiver);

    return (threads, receivers);
}

pub fn spin_thread(
    rvx: bus::BusReader<ThreadMessages>,
    live: bool,
    conn: postgres::Connection,
) -> (thread::JoinHandle<()>, Receiver<ThreadMessages>) {
    //
    let client = make_client();
    let (mut tpack, receiver) = ThreadPack::new(rvx, THIS_EXCHANGE);

    let listen_thread = thread::spawn(move || {
        if live {
            let products = get_products(&client);

            if products.len() > 0 {
                let mut ws_client = ws_connect();
                subscribe(&mut ws_client, products);

                iterate_client(tpack, ws_client, conn);
            }
            ()
        } else {
            tpack.message("Thread {:?} closing, not live.".to_string());
            tpack.notify_closed();
            ()
        }
    });

    return (listen_thread, receiver);
}

pub fn iterate_client(
    mut tpack: ThreadPack,
    mut client: websocket::client::sync::Client<Box<dyn NetworkStream + Send>>,
    conn: postgres::Connection,
) {
    // Check our messages.
    let mut do_close = false;
    while !do_close {
        for m in client.incoming_messages() {
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
            }

            if do_close {
                break;
            }
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

    database::notify_terminate(&conn, THIS_EXCHANGE);
    tpack.notify_closed();

    ()
}

fn ws_connect() -> websocket::sync::Client<Box<NetworkStream + Send>> {
    let mut headers = Headers::new();
    headers.set(UserAgent("hyper/0.5.2".to_owned()));

    let client = ClientBuilder::new(CONNECTION)
        .unwrap()
        .custom_headers(&headers)
        .connect(None)
        .unwrap();

    return client;
}

fn subscribe(
    client: &mut websocket::sync::Client<Box<NetworkStream + Send>>,
    products: Vec<String>,
) {
    let subscription = Subscription::new(products);
    let sub_json = serde_json::to_string_pretty(&subscription)
        .unwrap()
        .replace("kind", "type");
    println!("\n{}\n", sub_json);
    let m = client.send_message(&Message::text(sub_json));
    println!("{:?}", m);
}

fn make_client() -> hyper::Client {
    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);

    return client;
}

fn get_products(client: &hyper::Client) -> Vec<String> {
    let mut headers = Headers::new();
    headers.set(UserAgent("hyper/0.5.2".to_owned()));

    let mut resp = client
        .get("https://api.gdax.com/products")
        .headers(headers)
        .send()
        .unwrap();
    let mut body = vec![];
    resp.read_to_end(&mut body).unwrap();
    let resp = String::from_utf8_lossy(&body);
    let mut retval = vec![];

    let json: Vec<Value> = match serde_json::from_str(&resp) {
        Ok(x) => x,
        Err(e) => {
            println!("Product connection error: {:?}", e);
            println!("{:?}", resp);
            return retval;
        }
    };

    for j in json {
        match &j["id"] {
            Value::String(x) => retval.push(x.clone()),
            _ => (),
        }
        // println!("{:?}nn", j);
    }

    return retval;
}
