extern crate serde_json;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate chan;
extern crate chan_signal;

extern crate websocket;
extern crate stopwatch;
extern crate hyper;

use std::thread;
use websocket::{OwnedMessage};
use chan_signal::Signal;
// use stopwatch::{Stopwatch};

pub mod gemini;
pub mod database;

fn main() {
    //Set up channels
    let (s, r) = chan::sync(0);

    // Set up OS channel.
    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);

    // Allocate thread vector.
    let mut threads: Vec<thread::JoinHandle<()>> = Vec::new();

    // Spin threads.
    println!("Spinning threads...");
    threads.append(&mut gemini::start_gemini(s.clone(), r.clone()));

    // Await termination message.
    println!("Catching messages...");

    let mut looping = true;

    while looping {
        if !looping {
            println!("Looping is false, breaking?");
            break
        }

        chan_select!{
            default => (),
            r.recv() -> receipt => {
                match receipt {
                    Some(OwnedMessage::Close(m)) => {
                        println!("Main thread received close message: {:?}", m);
                        close_threads(&s, threads.len() as u8 - 1);
                        break
                    },
                    Some(OwnedMessage::Text(m)) => {
                        println!("Message to IO thread: {:?}", m);
                    },
                    None => continue,
                    _ => println!("Unknown message to main thread: {:?}", receipt)
                }
            },
            signal.recv() -> sig => {
                println!("OS Signal: {:?}", sig);
                close_threads(&s, threads.len() as u8);
                looping = false;
                break
            }
        }
    }

    // Clean up the threads.
    // thread::sleep(time::Duration::new(5, 0));
    println!("Waiting for threads to close...");

    let mut counter = 0;
    for thread in threads {
        println!("Trying to close thread {}", &counter);
        match thread.join() {
            Ok(_) => (),
            Err(e) => println!("Thread join: {:?}", e)
        }
        counter += 1;
    }

    println!("All done.");
}

fn close_threads(s: &chan::Sender<websocket::OwnedMessage>, num_threads: u8){
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
