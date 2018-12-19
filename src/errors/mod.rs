use std::error::Error;
use std::fmt;

pub enum CryptoError {
  Nothing,
  Restartable,
  NonRestartable,
}

#[derive(Debug)]
struct Restartable;

impl Error for Restartable {}

impl fmt::Display for Restartable {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Restartable error, spin up the client connection again.")
  }
}

#[derive(Debug)]
struct NonRestartable;

impl Error for NonRestartable {}

impl fmt::Display for NonRestartable {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Non-restartable error, terminate thread.")
  }
}
