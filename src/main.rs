#[macro_use]
extern crate serde_derive;

extern crate bus;
extern crate chan;
extern crate chan_signal;
extern crate hyper;
extern crate serde_json;
extern crate stopwatch;
extern crate websocket;

use bus::Bus;
use std::thread;
use websocket::OwnedMessage;
// use stopwatch::{Stopwatch};

// pub mod gemini;
// pub mod database;
pub mod gdax;

#[derive(Clone, Debug, PartialEq)]
pub enum ThreadMessages {
    Close,
    Greetings,
}

fn main() {
    //Set up channels
    let mut bus = Bus::new(10);
    let mut rvx = bus.add_rx();
    bus.broadcast(ThreadMessages::Greetings);
    println!("{:?}", rvx.recv());

    // Allocate thread vector.
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();

    // Spin threads.
    println!("Spinning threads...");
    // threads.append(&mut gemini::start_gemini(s.clone(), r.clone()));
    // threads.append(&mut gdax::start_gdax(s.clone(), r.clone()));

    // Await termination message.
    println!("Catching messages...");

    // loop {
    //
    //     chan_select!{
    //         default => (),
    //         r.recv() -> receipt => {
    //             match receipt {
    //                 Some(OwnedMessage::Close(m)) => {
    //                     println!("Main thread received close message: {:?}", m);
    //                     close_threads(&s, threads.len() as u8 - 1);
    //                     break
    //                 },
    //                 Some(OwnedMessage::Text(m)) => {
    //                     println!("Message to IO thread: {:?}", m);
    //                 },
    //                 None => continue,
    //                 _ => println!("Unknown message to main thread: {:?}", receipt)
    //             }
    //         },
    //         // signal.recv() -> sig => {
    //         //     println!("OS Signal: {:?}", sig);
    //         //     close_threads(&s, threads.len() as u8);
    //         //     break
    //         // }
    //     }
    // }

    // Clean up the threads.
    // thread::sleep(time::Duration::new(5, 0));
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

fn close_threads(s: &chan::Sender<websocket::OwnedMessage>, num_threads: u8) {
    for n in 0..num_threads {
        println!("Sending close message {}", n);
        // chan_select!{
        //     default => println!("Message sending failed on {}", n),
        //     s.send(OwnedMessage::Close(None)) => ()
        // }
        s.send(OwnedMessage::Close(None));
    }
    println!("Done sending close messages");
}
