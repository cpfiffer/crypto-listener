extern crate hyper;
extern crate hyper_native_tls;
extern crate postgres;
#[allow(non_snake_case)]
// #[feature(assoc_unix_epoch)]
extern crate serde;
extern crate serde_json;
extern crate websocket;

use crate::threadpack::*;
use crate::wspack;

use reqwest;
use std::sync::mpsc::Receiver;

// pub mod gdax_database;
pub mod types;

use self::types::*;

const CONNECTION: &'static str = "wss://ws-feed.gdax.com";
const THIS_EXCHANGE: &'static str = "gdax";
const SYMBOL_REQUEST: &'static str = "https://api.gdax.com/products";

pub fn extra_handling(message: String) -> String {
    return message;
}

pub fn start_gdax(
    rvx: bus::BusReader<ThreadMessages>,
    live: bool,
) -> (Vec<wspack::WSPack>, Vec<Receiver<ThreadMessages>>) {
    // Get clients and receivers.
    let (clients, receiver) = spin_thread(rvx, live);

    return (clients, vec![receiver]);
}

pub fn spin_thread(
    rvx: bus::BusReader<ThreadMessages>,
    live: bool,
) -> (Vec<wspack::WSPack>, Receiver<ThreadMessages>) {
    // Create receiver and client.
    let (mut gdaxclient, receiver) =
        wspack::WSPack::new(CONNECTION.to_string(), THIS_EXCHANGE, rvx, extra_handling);

    // Setup client.
    if live {
        let products = get_products();

        if products.len() > 0 {
            // Create subscription method.
            let sub_message = subscribe(products.clone());
            gdaxclient.set_subscription(sub_message);
        }
    }

    // Return values.
    return (vec![gdaxclient], receiver);
}

// pub fn iterate_client(
//     mut tpack: ThreadPack,
//     mut client: websocket::client::sync::Client<Box<dyn NetworkStream + Send>>,
//     conn: postgres::Connection,
// ) -> (ThreadPack, Result<(), CryptoError>) {
//     // Define error state.
//     let mut error_state = CryptoError::Nothing;

//     // Check our messages.
//     let mut do_close = false;
//     while !do_close {
//         for m in client.incoming_messages() {
//             match m {
//                 Ok(OwnedMessage::Close(_)) => {
//                     do_close = true;
//                     database::inject_log(
//                         &conn,
//                         format!("{} received termination from server.", tpack.exchange),
//                     );
//                 }
//                 Ok(OwnedMessage::Binary(_)) => {}
//                 Ok(OwnedMessage::Ping(_)) => {}
//                 Ok(OwnedMessage::Pong(_)) => {}
//                 Ok(OwnedMessage::Text(x)) => {
//                     database::inject_json(&conn, x);
//                 }
//                 Err(websocket::WebSocketError::NoDataAvailable) => {
//                     tpack.message(format!("{} - No data avilable.", THIS_EXCHANGE));
//                     error_state = CryptoError::Restartable;
//                     do_close = true;
//                 }
//                 Err(x) => {
//                     tpack.message(format!(
//                         "Error in {} receiving messages: {:?}",
//                         tpack.exchange, x
//                     ));
//                     do_close = true;
//                 }
//             }

//             // Check if we've been told to close.
//             if tpack.check_close() {
//                 tpack.message(format!("{} received close message.", tpack.exchange));
//                 error_state = CryptoError::NonRestartable;
//                 do_close = true;
//             }

//             if do_close {
//                 break;
//             }
//         }
//     }

//     // Wind the thread down.
//     let m_close = client.send_message(&Message::close());
//     match m_close {
//         Ok(_) => {}
//         Err(x) => {
//             let mess = format!(
//                 "Error closing client for thread {:?}, message: {}",
//                 thread::current().id(),
//                 x
//             );

//             tpack.message(mess.to_string());
//         }
//     }

//     database::notify_terminate(&conn, THIS_EXCHANGE);
//     tpack.notify_closed();

//     match error_state {
//         CryptoError::Nothing => return (tpack, Ok(())),
//         _ => return (tpack, Err(error_state)),
//     }
// }

// fn ws_connect() -> websocket::sync::Client<Box<NetworkStream + Send>> {
//     let mut headers = Headers::new();
//     headers.set(UserAgent("hyper/0.5.2".to_owned()));

//     let client = ClientBuilder::new(CONNECTION)
//         .unwrap()
//         .custom_headers(&headers)
//         .connect(None)
//         .unwrap();

//     return client;
// }

fn subscribe(products: Vec<String>) -> String {
    let subscription = Subscription::new(products);
    let sub_json = serde_json::to_string_pretty(&subscription)
        .unwrap()
        .replace("kind", "type");

    return sub_json;
}

fn get_products() -> Vec<String> {
    let body = reqwest::get(SYMBOL_REQUEST).unwrap().text().unwrap();
    let body: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    let mut products = vec![];

    for item in body.iter() {
        let id = match &item["id"] {
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

// Define a type so we can return multiple types of errors
enum FetchError {
    Http(hyper::Error),
    Json(serde_json::Error),
}

impl From<hyper::Error> for FetchError {
    fn from(err: hyper::Error) -> FetchError {
        FetchError::Http(err)
    }
}

impl From<serde_json::Error> for FetchError {
    fn from(err: serde_json::Error) -> FetchError {
        FetchError::Json(err)
    }
}
