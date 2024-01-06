use std::time::Duration;

#[derive(Clone, Debug)]
pub struct AsyncSocketOptions {
  pub path: String,
  pub port: u16,
  pub timeout: Duration,
}

impl AsyncSocketOptions {
  pub fn new() -> Self {
    Self {
      path: "test".to_string(),
      port: 2000,
      timeout: Duration::from_secs(1),
    }
  }

  pub fn with_path(self, path: &str) -> Self {
    Self {
      path: path.to_string(),
      ..self
    }
  }

  pub fn with_port(self, port: u16) -> Self {
    Self { port, ..self }
  }

  pub fn with_timeout(self, timeout: Duration) -> Self {
    Self { timeout, ..self }
  }
}

impl Default for AsyncSocketOptions {
  fn default() -> Self {
    Self::new()
  }
}
