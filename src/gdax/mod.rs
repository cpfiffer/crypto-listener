extern crate hyper;
extern crate hyper_native_tls;
extern crate postgres;
#[allow(non_snake_case)]
// #[feature(assoc_unix_epoch)]
extern crate serde;
extern crate serde_json;
extern crate websocket;

use self::hyper_native_tls::NativeTlsClient;
use self::postgres::Connection;
use super::ThreadMessages;
use chan::Receiver;
use chan::Sender;
use hyper::header::{Headers, UserAgent};
use hyper::net::HttpsConnector;
use hyper::Client;
use serde_json::Value;
use std::io::Read;
use std::thread;
use websocket::client::ClientBuilder;
use websocket::sync::stream::NetworkStream;
use websocket::{Message, OwnedMessage};

pub mod gdax_database;
pub mod types;

use self::gdax_database::*;
use self::types::*;

const CONNECTION: &'static str = "wss://ws-feed.gdax.com";

pub fn start_gdax(rvx: mut bus::BusReader<ThreadMessages>) -> Vec<thread::JoinHandle<()>> {
    // let connection_targets:
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();

    threads.push(spin_thread(rvx));

    return threads;
}

pub fn spin_thread(rvx: bus::BusReader<ThreadMessages>) -> thread::JoinHandle<()> {
    //
    let db_client = gdax_database::connect();

    let client = make_client();
    let listen_thread = thread::spawn(move || {
        let products = get_products(&client);

        if products.len() > 0 {
            let mut ws_client = ws_connect();
            subscribe(&mut ws_client, products);

            for message in ws_client.incoming_messages() {
                if let Ok(OwnedMessage::Text(x)) = message {
                    handle_message(x, &db_client);
                } else {
                    println!("Error incoming_messages: {:?}", message);
                    return;
                };

                match rvx.try_recv() {
                    Ok(ThreadMessages::Close) => {
                        return;
                    }
                    Ok(ThreadMessages::Greetings) => {}
                    Err(..) => {}
                }
            }
        }
        ()
    });

    return listen_thread;
}

fn handle_message(message: String, db_client: &Connection) {
    match inject(db_client, message) {
        Ok(_) => (),
        Err(e) => {
            println!("nMessage handling error: {:?}", e);
        }
    }
    // let message = message.replace("type", "kind");
    //
    // let parsed: Value = serde_json::from_str(&message).unwrap();
    //
    // let message_type = match &parsed["kind"] {
    //     Value::String(x) => x,
    //     _ => ""
    // };
    //
    // println!("nMessage: {:?}", &message);
    //
    // match message_type {
    //     "subscribe" => println!("{:?}", &message_type),
    //     "heartbeat" => println!("{:?}",Heartbeat::new(&message)),
    //     "received" => println!("{:?}",Received::new(&message)),
    //     "open" => println!("{:?}",Open::new(&message)),
    //     "done" => println!("{:?}",Done::new(&message)),
    //     "match" => println!("{:?}",Match::new(&message)),
    //     "change" => println!("{:?}",Change::new(&message)),
    //     "activate" => println!("{:?}",Activate::new(&message)),
    //     _ => println!("No message type: {}", &message_type),
    // }
    // println!("n");
}

fn ws_connect() -> websocket::sync::Client<Box<NetworkStream + Send>> {
    let mut headers = Headers::new();
    headers.set(UserAgent("hyper/0.5.2".to_owned()));

    let mut client = ClientBuilder::new(CONNECTION)
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
