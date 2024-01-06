use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Empty {
  _p: PhantomData<()>,
}

impl std::fmt::Display for Empty {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "()")
  }
}
