use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Clone, Debug, PartialEq)]
pub enum ThreadMessages {
  Close,
  Terminate,
  Greetings,
  Message(String),
}

pub struct ThreadPack {
  tcv: Sender<ThreadMessages>,
  broadcast_rcv: bus::BusReader<ThreadMessages>,
  pub exchange: &'static str,
}

impl ThreadPack {
  // Create a new threadpack. Returns a ThreadPack and a receiver for the
  // Threadpack to send messages back.
  pub fn new(
    b_receiver: bus::BusReader<ThreadMessages>,
    exchange: &'static str,
  ) -> (ThreadPack, Receiver<ThreadMessages>) {
    let (t, r) = mpsc::channel();
    let pack = ThreadPack {
      tcv: t,
      broadcast_rcv: b_receiver,
      exchange: exchange,
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

  // Notify the main thread to terminate everything.
  pub fn notify_abort(&mut self) {
    let _ = self.tcv.send(ThreadMessages::Terminate);
  }

  // Notify the main thread that the child is open.
  pub fn notify_open(&mut self) {
    let _ = self.tcv.send(ThreadMessages::Greetings);
  }

  pub fn check_close(&mut self) -> bool {
    let m = self.broadcast_rcv.try_recv();

    match m {
      Ok(ThreadMessages::Close) => {
        // The main thread told us to close, time to bail!
        // println!("check_close: {:?}", &m);
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
