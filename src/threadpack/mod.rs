use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Clone, Debug, PartialEq)]
pub enum ThreadMessages {
  Close,
  Greetings,
  Message(String),
}

pub struct ThreadPack {
  tcv: Sender<ThreadMessages>,
  broadcast_rcv: bus::BusReader<ThreadMessages>,
}

impl ThreadPack {
  // Create a new threadpack. Returns a ThreadPack and a receiver for the
  // Threadpack to send messages back.
  pub fn new(b_receiver: bus::BusReader<ThreadMessages>) -> (ThreadPack, Receiver<ThreadMessages>) {
    let (t, r) = mpsc::channel();
    let pack = ThreadPack {
      tcv: t,
      broadcast_rcv: b_receiver,
    };
    return (pack, r);
  }

  // Send a general message to the main thread.
  pub fn message(&mut self, message: String) {
    let _ = self.tcv.send(ThreadMessages::Message(message));
  }

  // Notify the main thread that the child is closing.
  pub fn notify_closed(&mut self) {
    let _ = self.tcv.send(ThreadMessages::Close);
  }

  // Notify the main thread that the child is open.
  pub fn notify_open(&mut self) {
    let _ = self.tcv.send(ThreadMessages::Greetings);
  }

  pub fn check_close(&mut self) -> bool {
    match self.broadcast_rcv.try_recv() {
      Ok(ThreadMessages::Close) => {
        // The main thread told us to close, time to bail!
        return true;
      }
      Ok(_) => {
        // Nothing to do here. Wave to the hello
        // as it passes us by.
      }
      Err(_) => {}
    }
    return false;
  }
}
