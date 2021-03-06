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
use crate::errors::CryptoError;
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

const CONNECTION: &'static str = "wss://api.bitfinex.com/ws";
const PAIRS_URI: &'static str = "https://api.bitfinex.com/v1/symbols";
const THIS_EXCHANGE: &'static str = "bitfinex";
const CHANNELS: &'static [&'static str] = &["trades", "book"];

pub fn start_bitfinex(
  rvx: bus::BusReader<ThreadMessages>,
  live: bool,
  password: String,
) -> (Vec<thread::JoinHandle<()>>, Vec<Receiver<ThreadMessages>>) {
  let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();
  let mut receivers: Vec<Receiver<ThreadMessages>> = Vec::new();

  let (handle, receiver) = spin_thread(rvx, live, password);
  threads.push(handle);
  receivers.push(receiver);

  return (threads, receivers);
}

pub fn spin_thread(
  rvx: bus::BusReader<ThreadMessages>,
  live: bool,
  password: String,
) -> (thread::JoinHandle<()>, Receiver<ThreadMessages>) {
  //
  let client = make_client();
  let (mut tpack, receiver) = ThreadPack::new(rvx, THIS_EXCHANGE);
  let products = get_products(&client);

  let listen_thread = thread::spawn(move || {
    if live {
      let products = get_products(&client);

      if products.len() > 0 {
        loop {
          let mut ws_client = ws_connect();
          let conn = database::connect(THIS_EXCHANGE, password.clone());

          subscribe(&mut ws_client, products.clone());
          let (t, result) = iterate_client(tpack, ws_client, conn);
          tpack = t;

          match result {
            Ok(_) => {
              tpack.message(format!(
                "{} closed with no error, restarting...",
                THIS_EXCHANGE
              ));
            }
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

              break;
            }
          }

          tpack.message(format!("{} is restarting.", THIS_EXCHANGE));
        }
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
) -> (ThreadPack, Result<(), CryptoError>) {
  // Define error state.
  let mut error_state = CryptoError::Nothing;

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
        Err(websocket::WebSocketError::NoDataAvailable) => {}
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
        error_state = CryptoError::NonRestartable;
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

  match error_state {
    CryptoError::Nothing => return (tpack, Ok(())),
    _ => return (tpack, Err(error_state)),
  }
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
  // println!("\n{}\n", sub_json);
  let _ = client.send_message(&Message::text(sub_json));
  // println!("{:?}", m);
}

fn make_client() -> hyper::Client {
  let ssl = NativeTlsClient::new().unwrap();
  let connector = HttpsConnector::new(ssl);
  let client = Client::with_connector(connector);

  return client;
}

// // request
// {
//    "event":"subscribe",
//    "channel":"book",
//    "pair":"<PAIR>",
//    "prec":"R0"
// }

// // response
// {
//    "event":"subscribed",
//    "channel":"book",
//    "chanId":"<CHANNEL_ID>",
//    "pair":"<PAIR>",
//    "prec":"R0",
//    "len":"<LENGTH>"
// }

fn get_products(client: &hyper::Client) -> Vec<String> {
  let mut headers = Headers::new();
  headers.set(UserAgent("hyper/0.5.2".to_owned()));

  let mut resp = client
    .get("https://api.bitfinex.com/v1/symbols")
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

  println!("{}", &json);

  for j in json {
    match &j["id"] {
      Value::String(x) => retval.push(x.clone()),
      _ => (),
    }
    // println!("{:?}nn", j);
  }

  return retval;
}
