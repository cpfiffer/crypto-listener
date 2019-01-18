#[macro_use]
extern crate serde_derive;
// #[macro_use]
// extern crate influx_db_client;
#[macro_use]
extern crate log;

extern crate bus;
extern crate ctrlc;
extern crate hyper;
extern crate pretty_env_logger;
extern crate serde_json;
extern crate stopwatch;
extern crate websocket;

use crate::threadpack::ThreadMessages;

use bus::Bus;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

// pub mod bitfinex;
pub mod database;
pub mod errors;
pub mod gdax;
// pub mod gemini;
// pub mod influx;
pub mod threadpack;
pub mod wspack;

fn main() {
    // Create logger.
    pretty_env_logger::init_timed();

    // Get arguments.
    let args: Vec<String> = env::args().collect();
    let live = args.contains(&"live".to_string());

    // Variables to handle CTRL + C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    //Set up channels
    let mut bus = Bus::new(10);

    // Allocate thread vector.
    // let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();
    let mut clients: Vec<wspack::WSPack> = Vec::new();
    let mut receivers: Vec<Receiver<threadpack::ThreadMessages>> = Vec::new();

    // Get password.
    // let password = "fuck".to_string(); //get_password();

    // Spin threads.
    info!("Spinning clients...");

    // Start Gemini
    // let (mut gemini_threads, mut gemini_receivers) = gemini::start_gemini(&mut bus, live.clone());
    // clients.append(&mut gemini_threads);
    // receivers.append(&mut gemini_receivers);

    // Start GDAX
    let (mut gdax_clients, mut gdax_receivers) = gdax::start_gdax(bus.add_rx(), live.clone());
    clients.append(&mut gdax_clients);
    receivers.append(&mut gdax_receivers);

    // Await termination message.
    info!("Catching messages...");

    // Set termination signal.
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    info!("Awaiting termination signal...");

    // Awaiting termination signal.
    let threads = handle_clients(clients, running, live, &mut bus, &mut receivers);

    let mut r2 = bus.add_rx();

    // Broadcast termination signal.
    bus.broadcast(threadpack::ThreadMessages::Terminate);

    info!("Main thread sent: {:?}", r2.try_recv());

    // Clean up the threads.
    info!("Waiting for threads...");

    let mut counter = 1;
    let total_threads = threads.len();
    for thread in threads {
        info!("Trying to close thread {} of {}", &counter, &total_threads);
        match thread {
            Ok(x) => match x.join() {
                Ok(_) => (),
                Err(e) => info!("Thread join: {:?}", e),
            },
            Err(e) => info!("Thread join: {:?}", e),
        }
        counter += 1;
    }

    info!("All done.");
}

fn handle_clients(
    mut clients: Vec<wspack::WSPack>,
    running: Arc<AtomicBool>,
    live: bool,
    bus: &mut bus::Bus<threadpack::ThreadMessages>,
    receivers: &mut Vec<Receiver<threadpack::ThreadMessages>>,
) -> Vec<Result<std::thread::JoinHandle<Result<(), errors::CryptoError>>, std::io::Error>> {
    // Allocate thread vector.
    let mut threads = vec![];

    // Do the loop.
    if live {
        // Make connections.
        for client in clients.iter_mut() {
            // Connect each client.
            let thread = client.handle(bus);
            threads.push(thread);
        }

        // Begin loop.
        while running.load(Ordering::SeqCst) {
            // Check receivers.
            for recv in &mut receivers.iter_mut() {
                match recv.try_recv() {
                    Ok(ThreadMessages::Message(m)) => {
                        info!("Notification: {}", m);
                    }
                    Ok(ThreadMessages::Greetings) => {}
                    Ok(ThreadMessages::Close) => {
                        // warn!("Notification: Received close.");
                    }
                    Ok(ThreadMessages::Terminate) => {
                        info!("Notification: Received abort signal, closing down.");
                        running.store(false, Ordering::SeqCst);
                        break;
                    }
                    Ok(ThreadMessages::Restart(uri)) => {
                        // Notify.
                        info!(
                            "Notification: Main thread received request to restart uri: {}",
                            &uri
                        );

                        // Find the WSPack with that URI.
                        for client in clients.iter_mut() {
                            if client.uri == uri {
                                // Connect.
                                let thread = client.handle(bus);
                                threads.push(thread);
                                info!(
                                    "Notification: Main thread found and restarted uri: {}",
                                    &uri
                                );
                                break;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }
    }

    return threads;
}

// Returns true if any receiver has an error.
// fn check_receivers(receivers: &Vec<Receiver<threadpack::ThreadMessages>>) -> bool {
//     for i in receivers.iter() {
//         match i.try_recv() {
//             Ok(ThreadMessages::Message(m)) => {
//                 info!("Receivers: {:?}", m);
//             }
//             Ok(ThreadMessages::Greetings) => {}
//             Ok(ThreadMessages::Close) => {
//                 info!("Receivers: Received close.");
//             }
//             Ok(ThreadMessages::Terminate) => {
//                 info!("Receivers: Received abort signal, closing down.");
//                 return true;
//             }
//             Err(_) => {}
//         }
//     }

//     return false;
// }

// fn get_password() -> String {
//     let mut s = String::new();
//     print!("Enter PSQL password: ");
//     let _ = stdout().flush();
//     stdin()
//         .read_line(&mut s)
//         .expect("Did not enter a correct string");
//     if let Some('\n') = s.chars().next_back() {
//         s.pop();
//     }
//     if let Some('\r') = s.chars().next_back() {
//         s.pop();
//     }
//     return s;
// }
