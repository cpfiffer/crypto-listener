#[derive(PartialEq)]
pub enum CryptoError {
  Nothing,
  Restartable,
  NonRestartable,
  UninitializedClient,
}
