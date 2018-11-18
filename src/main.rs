#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate influx_db_client;

extern crate bus;
extern crate ctrlc;
extern crate hyper;
extern crate serde_json;
extern crate stopwatch;
extern crate websocket;

use bus::Bus;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

// pub mod gemini;
pub mod gdax;
pub mod influx;

#[derive(Clone, Debug, PartialEq)]
pub enum ThreadMessages {
    Close,
    Greetings,
}

fn main() {
    // Variables to handle CTRL + C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    //Set up channels
    let mut bus = Bus::new(10);
    bus.broadcast(ThreadMessages::Greetings);

    // Allocate thread vector.
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();

    // Spin threads.
    println!("Spinning threads...");
    // threads.append(&mut gemini::start_gemini(s.clone(), r.clone()));
    threads.append(&mut gdax::start_gdax(bus.add_rx()));
    // Await termination message.
    println!("Catching messages...");

    // Set termination signal.
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    println!("Awaiting termination signal...");

    // Awaiting termination signal.
    while running.load(Ordering::SeqCst) {}

    // Broadcast termination signal.
    bus.broadcast(ThreadMessages::Close);

    // Clean up the threads.
    println!("Waiting for threads to close...");

    let mut counter = 0;
    for thread in threads {
        println!("Trying to close thread {}", &counter);
        match thread.join() {
            Ok(_) => (),
            Err(e) => println!("Thread join: {:?}", e),
        }
        counter += 1;
    }

    println!("All done.");
}
