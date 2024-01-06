pub trait SerMessage: ::serde::ser::Serialize {}

pub trait DeMessage: ::serde::de::DeserializeOwned {}
