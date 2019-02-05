extern crate postgres;
extern crate websocket;

use crate::threadpack::*;
use crate::wspack::WSPack;

use serde_json;
use hyper;
use std::sync::mpsc::Receiver;

// Connection strings.
const CONNECTION: &'static str = "wss://api.gemini.com/v1/marketdata/";
const THIS_EXCHANGE: &'static str = "gemini";
const SYMBOL_REQUEST: &'static str = "https://api.gemini.com/v1/symbols";

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
        for pair in products {
            // Get URI.
            let target = format!("{}{}", CONNECTION, &pair);

            // Make a client.
            let (new_client, rx) = WSPack::new(target, THIS_EXCHANGE, rvx.add_rx(), pair.clone());

            // Stash the new client and receiver.
            clients.push(new_client);
            receivers.push(rx);
        }
    }

    // Return.
    return (clients, receivers);
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
