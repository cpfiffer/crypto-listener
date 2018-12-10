#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate influx_db_client;
#[macro_use]
extern crate log;

extern crate bus;
extern crate ctrlc;
extern crate hyper;
extern crate pretty_env_logger;
extern crate serde_json;
extern crate stopwatch;
extern crate websocket;

use bus::Bus;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread;
use threadpack::*;

pub mod gdax;
pub mod gemini;
pub mod influx;
pub mod threadpack;

fn main() {
    // Create logger.
    pretty_env_logger::init();

    // Get arguments.
    let args: Vec<String> = env::args().collect();
    let live = args.contains(&"live".to_string());

    // Variables to handle CTRL + C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    //Set up channels
    let mut bus = Bus::new(0);
    bus.broadcast(ThreadMessages::Greetings);

    // Allocate thread vector.
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();
    let mut receivers: Vec<Receiver<ThreadMessages>> = Vec::new();

    // Spin threads.
    println!("Spinning threads...");

    // Start gemini
    let (mut gemini_threads, mut gemini_receivers) = gemini::start_gemini(&mut bus, live.clone());
    threads.append(&mut gemini_threads);
    receivers.append(&mut gemini_receivers);

    // Start GDAX
    let (mut gdax_threads, mut gdax_receivers) = gdax::start_gdax(bus.add_rx(), live.clone());
    threads.append(&mut gdax_threads);
    receivers.append(&mut gdax_receivers);

    // Await termination message.
    println!("Catching messages...");

    // Set termination signal.
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    println!("Awaiting termination signal...");

    // Awaiting termination signal.
    while running.load(Ordering::SeqCst) {
        // Check if any threads are busted.
        if check_receivers(&receivers) {
            break;
        }
    }

    // Broadcast termination signal.
    bus.broadcast(ThreadMessages::Close);

    // Clean up the threads.
    println!("Waiting for threads...");

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

// Returns true if any receiver has an error.
fn check_receivers(receivers: &Vec<Receiver<ThreadMessages>>) -> bool {
    for i in receivers.iter() {
        match i.try_recv() {
            Ok(m) => {
                println!("Checking receivers: {:?}", m);
                return true;
            }
            Err(_) => {}
        }
    }

    return false;
}
