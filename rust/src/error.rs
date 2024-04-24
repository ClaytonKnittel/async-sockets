use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::util::Empty;

pub type AsyncSocketResult<T = ()> = Result<T, Status>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
  tag = "status",
  content = "payload",
  bound(serialize = "T: Serialize", deserialize = "T: DeserializeOwned")
)]
pub enum Status<T = Empty> {
  Ok(T),

  /// Serialization errors.
  SerializeFailed(String),
  DeserializeFailed(String),

  /// Async messaging errors.
  MessageQueueError(String),
  RemoveMessageQueueError(String),
  RecvError(String),
  Timeout(String),

  /// Internal server errors.
  InternalServerError(String),
}

impl<T> Status<T> {
  pub fn clear_value(self) -> Status<Empty> {
    match self {
      Status::Ok(_) => Status::Ok(Empty::default()),
      Status::SerializeFailed(msg) => Status::DeserializeFailed(msg),
      Status::DeserializeFailed(msg) => Status::DeserializeFailed(msg),
      Status::MessageQueueError(msg) => Status::MessageQueueError(msg),
      Status::RemoveMessageQueueError(msg) => Status::RemoveMessageQueueError(msg),
      Status::RecvError(msg) => Status::RecvError(msg),
      Status::Timeout(msg) => Status::Timeout(msg),
      Status::InternalServerError(msg) => Status::InternalServerError(msg),
    }
  }
}

impl<T: std::fmt::Debug + std::fmt::Display> std::error::Error for Status<T> {}

impl<T: std::fmt::Display> std::fmt::Display for Status<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Status::Ok(val) => {
        write!(f, "Ok({val})")
      }
      Status::SerializeFailed(message) => {
        write!(f, "Failed to serialize: {message}")
      }
      Status::DeserializeFailed(message) => {
        write!(f, "Failed to deserialize: {message}")
      }
      Status::MessageQueueError(message) => {
        write!(f, "Failed to queue message: {message}")
      }
      Status::RemoveMessageQueueError(message) => {
        write!(f, "Failed to remove queued message: {message}")
      }
      Status::RecvError(message) => {
        write!(f, "Recv error on channel: {message}")
      }
      Status::Timeout(message) => {
        write!(f, "Timeout: {message}")
      }
      Status::InternalServerError(message) => {
        write!(f, "Internal server error: {message}")
      }
    }
  }
}

/// TODO: get rid of this and impl std::ops::Try for Status once
/// https://github.com/rust-lang/rust/issues/84277 is merged.
pub type KokoResult<T = ()> = Result<T, Status>;
