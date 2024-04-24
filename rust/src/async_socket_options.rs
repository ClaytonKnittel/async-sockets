use std::{
  net::{SocketAddr, SocketAddrV4},
  time::Duration,
};

#[derive(Clone, Debug)]
pub struct AsyncSocketSecurity {
  pub cert_path: String,
  pub key_path: String,
}

#[derive(Clone, Debug)]
pub struct AsyncSocketOptions {
  pub path: String,
  pub bind_addr: SocketAddr,
  pub timeout: Duration,
  pub verbose: bool,
  pub security: Option<AsyncSocketSecurity>,
}

impl AsyncSocketOptions {
  pub fn new() -> Self {
    Self {
      path: "test".to_string(),
      bind_addr: SocketAddr::V4(SocketAddrV4::new([127, 0, 0, 1].into(), 2000)),
      timeout: Duration::from_secs(1),
      verbose: false,
      security: None,
    }
  }

  pub fn with_path(self, path: &str) -> Self {
    Self {
      path: path.to_string(),
      ..self
    }
  }

  pub fn with_bind_addr(self, bind_addr: SocketAddr) -> Self {
    Self { bind_addr, ..self }
  }

  pub fn with_timeout(self, timeout: Duration) -> Self {
    Self { timeout, ..self }
  }

  pub fn with_verbose(self, verbose: bool) -> Self {
    Self { verbose, ..self }
  }

  pub fn with_security(self, security: AsyncSocketSecurity) -> Self {
    Self {
      security: Some(security),
      ..self
    }
  }
}

impl Default for AsyncSocketOptions {
  fn default() -> Self {
    Self::new()
  }
}
