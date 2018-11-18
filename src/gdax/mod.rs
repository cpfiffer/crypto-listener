extern crate hyper;
extern crate hyper_native_tls;
extern crate postgres;
#[allow(non_snake_case)]
// #[feature(assoc_unix_epoch)]
extern crate serde;
extern crate serde_json;
extern crate websocket;

use self::hyper_native_tls::NativeTlsClient;
use super::ThreadMessages;
use hyper::header::{Headers, UserAgent};
use hyper::net::HttpsConnector;
use hyper::Client;
use influx;
use serde_json::Value;
use std::io::Read;
use std::thread;
use websocket::client::ClientBuilder;
use websocket::sync::stream::NetworkStream;
use websocket::{Message, OwnedMessage};

// pub mod gdax_database;
pub mod types;

use self::types::*;

const CONNECTION: &'static str = "wss://ws-feed.gdax.com";
const THIS_EXCHANGE: &'static str = "gdax";

pub fn start_gdax(rvx: bus::BusReader<ThreadMessages>) -> Vec<thread::JoinHandle<()>> {
    // let connection_targets:
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();

    threads.push(spin_thread(rvx));

    return threads;
}

pub fn spin_thread(mut rvx: bus::BusReader<ThreadMessages>) -> thread::JoinHandle<()> {
    //
    let client = make_client();
    let listen_thread = thread::spawn(move || {
        let products = get_products(&client);

        if products.len() > 0 {
            let mut ws_client = ws_connect();
            subscribe(&mut ws_client, products);

            for message in ws_client.incoming_messages() {
                if let Ok(OwnedMessage::Text(x)) = message {
                    handle_message_influx(x, THIS_EXCHANGE);
                } else {
                    println!("Error gdax incoming_messages: {:?}", message);
                    // break;
                };

                match rvx.try_recv() {
                    Ok(y @ ThreadMessages::Close) => {
                        println!(
                            "Thread {:?} received message {:?}",
                            thread::current().id(),
                            y
                        );
                        break;
                    }
                    Ok(y @ ThreadMessages::Greetings) => {
                        println!(
                            "Thread {:?} received message {:?}",
                            thread::current().id(),
                            y
                        );
                    }
                    Err(..) => {}
                }
            }

            let m_close = ws_client.send_message(&Message::close());
            match m_close {
                Ok(_) => {}
                Err(x) => {
                    println!(
                        "Error closing client for thread {:?}, message: {}",
                        thread::current().id(),
                        x
                    );
                }
            }

            println!("Thread {:?} closing.", thread::current().id());
        }
        ()
    });

    return listen_thread;
}

// THIS IS FOR
// USING A REAL
// DATABSE
// pub fn spin_thread(mut rvx: bus::BusReader<ThreadMessages>) -> thread::JoinHandle<()> {
//     //
//     let db_client = gdax_database::connect();

//     let client = make_client();
//     let listen_thread = thread::spawn(move || {
//         let products = get_products(&client);

//         if products.len() > 0 {
//             let mut ws_client = ws_connect();
//             subscribe(&mut ws_client, products);

//             for message in ws_client.incoming_messages() {
//                 if let Ok(OwnedMessage::Text(x)) = message {
//                     handle_message(x, &db_client);
//                 } else {
//                     println!("Error incoming_messages: {:?}", message);
//                     return;
//                 };

//                 match rvx.try_recv() {
//                     Ok(y) => {
//                         println!(
//                             "Thread {:?} received message {:?}",
//                             thread::current().id(),
//                             y
//                         );
//                     }
//                     Err(..) => {}
//                 }
//             }
//         }
//         ()
//     });

//     return listen_thread;
// }

fn handle_message_influx(message: String, exchange: &'static str) {
    influx::inject_influx(message, exchange);
}

// fn handle_message(message: String, db_client: &Connection) {
//     match inject(db_client, message) {
//         Ok(_) => (),
//         Err(e) => {
//             println!("nMessage handling error: {:?}", e);
//         }
//     }
// }

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
