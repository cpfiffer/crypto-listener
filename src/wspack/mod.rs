use crate::database;
use crate::errors::CryptoError;
use crate::threadpack;
use crate::threadpack::ThreadMessages;

use bus;
use postgres;
use std::sync::mpsc::Receiver;
use std::thread;
use std::collections::HashMap;
use websocket::client::sync::Client;
use websocket::client::ClientBuilder;
use websocket::header::Headers;
use websocket::header::UserAgent;
use websocket::stream::sync::NetworkStream;
use websocket::Message;
use websocket::OwnedMessage;
// use std::time::Duration;
// use std::time::SystemTime;

// Constants
// const MESSAGE_TIMEOUT: Duration = Duration::from_secs(360);

// A WSPack holds a URI, a database connection, and contains
// procedures to start and stop a websocket connection.
pub struct WSPack {
  pub exchange: &'static str,
  pub uri: String,
  tpack: threadpack::ThreadPack,
  headers: websocket::header::Headers,
  subscription_message: String,
  pub pair: String
}

impl WSPack {
  // Default useragent headers.
  fn default_headers() -> Headers {
    let mut headers = Headers::new();
    headers.set(UserAgent("hyper/0.5.2".to_owned()));

    return headers;
  }

  // Set a subscription to use when connecting.
  pub fn set_subscription(&mut self, message: String) {
    info!("{} set subscription message: \n{:?}", &self.uri, &message);

    self.subscription_message = message;
  }

  // Make a new webpack.
  pub fn new(
    uri: String,
    exchange: &'static str,
    b_receiver: bus::BusReader<ThreadMessages>,
    pair: String
  ) -> (WSPack, Receiver<threadpack::ThreadMessages>) {
    let (tpack, receiver) = threadpack::ThreadPack::new(b_receiver, exchange);

    let ws = WSPack {
      exchange: exchange,
      uri: uri,
      tpack: tpack,
      headers: WSPack::default_headers(),
      subscription_message: String::new(),
      pair: pair
    };

    return (ws, receiver);
  }

  // Connect to the websocket.
  pub fn connect(
    &mut self,
    bus: &mut bus::Bus<ThreadMessages>,
  ) -> (
    postgres::Connection,
    Client<Box<dyn NetworkStream + Send>>,
    String,
    threadpack::ThreadPack,
    HashMap<String, String>
  ) {
    // Connect to the database.
    let conn = database::connect(&self.exchange);

    // Stick the database in its slot.
    // self.conn = Some(conn);

    // Connect to the URI.
    let client = ClientBuilder::new(&self.uri)
      .unwrap()
      .custom_headers(&self.headers)
      .connect(None)
      .unwrap();

    // Stick the client in the slot.
    // self.client = Some(client);

    // Notify the main thread that we connected.
    self.message(format!("Successfully connected to {}!", &self.uri));

    // Log our connection on the database.
    database::inject_log(&conn, format!("Connected to {}", self.uri));

    // Send the subscription method if we've got one.
    let mess = self.subscription_message.clone();

    // Create a hashmap of stuff to inject.
    let mut injector = HashMap::new();

    if self.pair.len() > 0 {
      injector.insert("CRYPTOPAIR".to_string(), self.pair.clone());
    }

    // if mess.len() > 0 {
    //   &self.send_json(&mess);
    // }

    return (conn, client, mess, self.tpack.clone(bus), injector);
  }

  // Message the main thread.
  pub fn message(&mut self, message: String) {
    info!("{}", message);
    // self.tpack.message(message);
  }

  // Handle incoming messages.
  pub fn handle(
    &mut self,
    bus: &mut bus::Bus<ThreadMessages>,
  ) -> Result<thread::JoinHandle<Result<(), CryptoError>>, std::io::Error> {
    // Retrieve the database connection, the WS client, and subscription message.
    let (conn, mut client, sub_message, mut tpack, injector) = self.connect(bus);
    let uri = self.uri.clone();

    // Send the subscription message.
    if sub_message.len() > 0 {
      let _ = client.send_message(&Message::text(sub_message));
    }

    // Iniate listening thread.
    let thread = thread::Builder::new().name(uri.clone()).spawn(move || {
      // Count times we have received the "NoData" error.
      // let mut last_message = SystemTime::now();

      // Wait for a couple seconds to ensure closure.
      thread::sleep(std::time::Duration::from_secs(3));

      // Set initial error states.
      let mut error_state = CryptoError::Nothing;

      for m in &mut client.incoming_messages() {
        // match m {
        //   Err(websocket::WebSocketError::NoDataAvailable) => {},
        //   _ => debug!("{:?}", &m)
        // }
        
        match m {
          Ok(OwnedMessage::Close(_)) => {
            error_state = CryptoError::Restartable;
            tpack.notify_closed();
          }
          Ok(OwnedMessage::Binary(x)) |
          Ok(OwnedMessage::Ping(x)) |
          Ok(OwnedMessage::Pong(x)) => {
            tpack.message(format!("URI: {} received non-text message: {:?}", &uri, x));
          }
          Ok(OwnedMessage::Text(x)) => {
            // last_message = SystemTime::now();
            database::inject_json(&conn, x.to_string(), &injector);
          }
          Err(websocket::WebSocketError::NoDataAvailable) => {
            // Just close the damn thread.
            error_state = CryptoError::Restartable;
                tpack.message(format!("URI: {} has not received has no data available. Closing thread.", &uri));
                tpack.notify_closed();

            // Check time since last receipt.
            // let now = SystemTime::now();
            // if let Ok(duration) = now.duration_since(last_message) {
            //   if duration >= MESSAGE_TIMEOUT {
            //     error_state = CryptoError::Restartable;
            //     tpack.message(format!("URI: {} has not received a message in {:?}. Restarting thread.", &uri, duration));
            //     tpack.notify_closed();
            //   }
            // }
          }
          Err(websocket::WebSocketError::IoError(e)) => {
            tpack.message(format!("IOError {} : {:?}", &uri, e));
            error_state = CryptoError::Restartable;
          },
          Err(x) => {
tpack.message(format!("Error in {} receiving messages: {:?}", &uri, x));
            error_state = CryptoError::Restartable;
            tpack.notify_abort();
        }
        }

        // Check if we've been told to close.
        if tpack.check_close() {
          println!("{} received close message.", &uri);
          tpack.notify_abort();
          error_state = CryptoError::NonRestartable;
        }

        // Check our error state.
        match error_state {
          CryptoError::Nothing => {},
          CryptoError::Restartable |
          CryptoError::NonRestartable |
          CryptoError::UninitializedClient => break,
        }
      }

      // Close connections.
      let _ = conn.finish();
      let _ = client.send_message(&Message::close());

      // Notify
      tpack.message(format!("Terminated connection to {}!", &uri));

      if error_state == CryptoError::Restartable {
        tpack.notify_restart(uri.clone());
      }

      // Thread return.
      match error_state {
        CryptoError::Nothing => return Ok(()),
        _ => return Err(error_state),
      }
    });

    return thread;
  }
}
