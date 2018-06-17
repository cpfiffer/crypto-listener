#[allow(non_snake_case)]
// #[feature(assoc_unix_epoch)]

extern crate serde;
extern crate serde_json;
extern crate hyper;
extern crate websocket;
extern crate postgres;

use serde_json::Value::{Null};
use serde_json::{Value, Error};
use chan::Receiver;
use chan::Sender;
use std::thread;
use std::time;
use websocket::{Message, OwnedMessage};
use websocket::client::ClientBuilder;
use super::database;
use self::postgres::Connection;
use hyper::header::{Headers};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod types;

use self::types::*;

const CONNECTION: &'static str = "wss://ws-feed.gdax.com";

pub fn start_gdax(s: Sender<websocket::OwnedMessage>,
                   r: Receiver<websocket::OwnedMessage>) -> Vec<thread::JoinHandle<()>> {
    // let connection_targets:
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();

    threads.push(spin_thread(s.clone(), r.clone()));

    return threads;
}

pub fn spin_thread(
                   s: Sender<websocket::OwnedMessage>,
                   r: Receiver<websocket::OwnedMessage>) -> thread::JoinHandle<()> {
    //
    let listen_thread = thread::spawn(move || ());

    return listen_thread;
}
