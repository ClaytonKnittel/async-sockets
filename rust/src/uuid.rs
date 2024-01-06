use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Uuid {
  uuid: ::uuid::Uuid,
}

impl Uuid {
  pub fn new_v4() -> Self {
    Self {
      uuid: ::uuid::Uuid::new_v4(),
    }
  }
}

impl std::fmt::Display for Uuid {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.uuid)
  }
}

impl Serialize for Uuid {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    self.uuid.to_string().serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for Uuid {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    Ok(Self {
      uuid: ::uuid::Uuid::try_parse(<&str>::deserialize(deserializer)?)
        .map_err(|err| D::Error::custom(format!("Failed to parse Uuid from string: {err}")))?,
    })
  }
}
