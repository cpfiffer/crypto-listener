#![allow(non_snake_case)]

extern crate serde;
extern crate serde_json;
extern crate hyper;
extern crate websocket;
extern crate postgres;

use serde_json::{Value};
use chan::Receiver;
use chan::Sender;
use std::thread;
use websocket::{OwnedMessage};
use websocket::client::ClientBuilder;
use super::database;
use self::postgres::Connection;
use hyper::header::{Headers};
use std::time::{SystemTime, UNIX_EPOCH};

// Connection strings.
const CONNECTION: &'static str = "wss://api.gemini.com/v1/marketdata/";
const PAIRS: &'static [&'static str] = &[
    "BTCUSD",
    "ETHUSD",
    "ETHBTC",
    "ZECUSD",
    "ZECBTC",
    "ZECETH"
];

pub fn start_gemini(s: Sender<websocket::OwnedMessage>,
                   r: Receiver<websocket::OwnedMessage>) -> Vec<thread::JoinHandle<()>> {
    // let connection_targets:
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();

    for pair in PAIRS {
        threads.push(spin_thread(pair, s.clone(), r.clone()))
    }

    return threads;
}

pub fn spin_thread(pair: &'static str,
                   s: Sender<websocket::OwnedMessage>,
                   r: Receiver<websocket::OwnedMessage>) -> thread::JoinHandle<()> {
    // Spin up some threads and return them.
    let target = [CONNECTION, pair].join("");
    println!("Connecting to {}...", &target);

    let mut client = ClientBuilder::new(&target)
        .unwrap()
        .connect(None)
        .unwrap();

    println!("Successfully connected to {}!", &target);

    let conn = database::connect();

    // Wait for messages back.
    let listen_thread = thread::spawn(move || {
        let mut socket_counter = 0;

        for message in client.incoming_messages() {
            chan_select!{
                default => (),
                r.recv() -> receipt => {
                    // println!("Closing thread {}: {:?}", &pair, receipt);
                    return
                },
            }

            let message = match message {
                Ok(websocket::OwnedMessage::Close(m)) => {
                    println!("Thread for {} received a close message: {:?}", &target, m);
                    return;
                },
                Ok(m) => m,
                Err(e) => {
                    println!("Receive loop for {:?}: {:?}", &target, e);
                    return;
                },
            };

            // Pull the string out of the message.
            let message_text = match message {
                OwnedMessage::Text(m) => m,
                _ => String::new()
            };

            // Handle the string.
            let message_socket = handle_message(&message_text, &conn, pair);

            // Check the socket counter.
            if message_socket == socket_counter {
                //
                socket_counter += 1;
            }
            else {
                s.send(OwnedMessage::Close(None));
                println!("Closing pair {:?} due to socket mismatch. We have {} and we received {}.",
                    &pair,
                    &socket_counter,
                    &message_socket);
                break;
            }
        }
        client.send_message(&OwnedMessage::Close(None)).unwrap();
        // println!("Thread {} closed.", &pair);
    });

    return listen_thread;
}

pub fn gemini_headers() -> Headers{
    let mut headers = Headers::new();

    headers.append_raw("X-GEMINI-APIKEY", b"fuck".to_vec());
    headers.append_raw("X-GEMINI-PAYLOAD", b"fuck".to_vec());
    headers.append_raw("X-GEMINI-SIGNATURE", b"fuck".to_vec());

    return headers;
}

fn handle_message(json: &str, conn: &Connection, pair: &'static str) -> u32 {
    // Replace type with kind. This uses serde_json, which deserializes into
    // strongly typed structs by field name -- this means that I have to change the names
    // of the string to allow serde to deserialize it.
    let json = json.replace("type", "kind");

    let mut v = match serde_json::from_str(&json.to_string()) {
        Ok(x)  => x,
        Err(e) => {
            let truncated = &json[0..100];
            println!("{:?}", e);
            println!("\nValue:\n {:?}", truncated);
            serde_json::Value::Null
        }
    };

    // Get the socket sequence.
    let sock = match &v["socket_sequence"] {
        Value::Number(n) => n.as_u64().unwrap(),
        _ => 0
    };

    //Find out what type of update this is.
    let v_type = update_type(&v);

    //Act on the type of udpates we got.
    match v_type {
        | UpdateType::Heartbeat  => {
            // Handle heartbeat.

            // Increment socket counter.
        },
        | UpdateType::Update | UpdateType::Initial    => {
            // Handle update.

            // If initial, add a timestamp.
            match v_type {
                UpdateType::Initial => {
                    let ts = timestamp();
                    let tsm = timestampms();

                    v["timestampms"] = {
                        let num = serde_json::Number::new(tsm);
                        Value::Number(num)
                    };

                    v["timestamp"] = {
                        let num = serde_json::Number::new(ts);
                        Value::Number(num)
                    };
                },
                _ => ()
            }

            // Convert to struct
            let mut update: Update = match serde_json::from_value(v) {
                Ok(x) => x,
                Err(e) => {
                    println!("Error parsing update to struct. Error: {:?}", e);
                    let truncated = &json[0..100];
                    println!("\nValue:\n {:?}", truncated);
                    return 0;
                }
            };

            // Foreach value object, deserialize it.
            let mut trades: Vec<Trade> = Vec::new();
            let mut changes: Vec<Change> = Vec::new();

            for event in update.events.clone() {
                let event_type = change_type(&event);

                match event_type {
                    ChangeType::Change => {
                        let ev: Change = match serde_json::from_value(event){
                            Ok(o) => o,
                            Err(e) => {
                                println!("Error parsing event: {:?}", e);
                                return 0;
                            }
                        };
                        changes.push(ev);
                    },
                    ChangeType::Trade  => {
                        let td: Trade = match serde_json::from_value(event) {
                            Ok(o) => o,
                            Err(e) => {
                                println!("Error parsing event: {:?}", e);
                                return 0;
                            }
                        };
                        trades.push(td);
                    }
                    ChangeType::Null   => println!("\nUnknown change type: {:?}", &event),
                }
            }

            database::inject_gemini(&conn, update, trades, changes, pair).unwrap();
        },
        UpdateType::Null => ()//println!("\nUnknown message type: {:?}\n", &v)
    }

    return sock as u32;
}

enum UpdateType {
    Heartbeat,
    Update,
    Initial,
    Null
}

enum ChangeType {
    Change,
    Trade,
    Null
}

fn update_type(value :&Value) -> UpdateType {
    let text_val = match value["kind"] {
        Value::String(ref f) => f,
        _ => ""
    };

    let timestamp_val = match value["timestampms"] {
        Value::Number(ref f) => f.as_i64().unwrap(),
        _ => -1
    };

    let kind = match text_val {
        "update" => {
            if timestamp_val > 0 {
                UpdateType::Update
            }
            else {
                UpdateType::Initial
            }
        },
        "heartbeat" => UpdateType::Heartbeat,
        _ => UpdateType::Null
    };

    return kind
}

fn change_type(value :&Value) -> ChangeType {
    let text_val = match value["kind"] {
        Value::String(ref f) => f,
        _ => ""
    };

    let kind = match text_val {
        "change" => ChangeType::Change,
        "trade" => ChangeType::Trade,
        _ => ChangeType::Null
    };

    return kind
}

// Structs for deserializing JSON
#[derive(Serialize, Deserialize, Debug)]
pub struct Update {
    pub kind: String,
    pub timestampms: i64,
    pub timestamp: u32,
    pub socket_sequence: u32,
    pub eventId: u32,
    pub events: Vec<Value>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Change {
    pub kind: String,
    pub reason: String,
    pub price: String,
    pub delta: String,
    pub remaining: String,
    pub side: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Trade {
    pub kind: String,
    pub tid: u32,
    pub price: String,
    pub amount: String,
    pub makerSide: String
}

fn timestampms() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let in_ms = since_the_epoch.as_secs() * 1000 + since_the_epoch.subsec_nanos() as u64 / 1_000_000;
    return in_ms;
}

fn timestamp() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let in_s = since_the_epoch.as_secs();
    return in_s;
}
