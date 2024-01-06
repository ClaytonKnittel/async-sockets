use gen_emitters::gen_emitters;
use gen_listeners::gen_listeners;
use gen_responders::gen_responders;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, ItemEnum};

mod gen_emitters;
mod gen_listeners;
mod gen_responders;
mod util;

/// Derive macro for any type of message that is to be sent over the WebSocket.
/// Must be used on an enum of all messages of a particular type (emit/call),
/// and defines `::serde::ser::Serialize` for the enum.
///
/// Example:
/// ```
/// #[derive(async_socket_macros::AsyncSocketEmitters)]
/// enum MyEmitEvents {
///   TestMessage { id: u64, name: String },
///   TestMessage2 { },
/// }
/// ```
#[proc_macro_error]
#[proc_macro_derive(AsyncSocketEmitters)]
pub fn derive_async_socket_emitters(input: TokenStream) -> TokenStream {
  let item = parse_macro_input!(input as ItemEnum);

  gen_emitters(item).into()
}

/// Derive macro for response messages to `call` messages from the client, that
/// are to be sent over the WebSocket. Must be used on an enum of all response
/// messages, and defines `::serde::ser::Serialize` for the enum.
///
/// Example:
/// ```
/// #[derive(AsyncSocketResponders)]
/// enum MyRespondEvents {
///   TestMessage { id: u64, name: String },
///   TestMessage2 { },
/// }
/// ```
#[proc_macro_error]
#[proc_macro_derive(AsyncSocketResponders)]
pub fn derive_async_socket_responders(input: TokenStream) -> TokenStream {
  let item = parse_macro_input!(input as ItemEnum);

  gen_responders(item).into()
}

/// Derive macro for any type of message that is to be received from the
/// WebSocket. Must be used on an enum of all messages of a particular type
/// (emit listener/response), and defines `::serde::de::DeserializeOwned` for
/// the enum.
///
/// Example:
/// ```
/// #[derive(AsyncSocketListeners)]
/// enum TheirEmitEvents {
///   TestMessage { id: u64, name: String },
///   TestMessage2 { },
/// }
/// ```
#[proc_macro_error]
#[proc_macro_derive(AsyncSocketListeners)]
pub fn derive_async_socket_listeners(input: TokenStream) -> TokenStream {
  let item = parse_macro_input!(input as ItemEnum);

  gen_listeners(item).into()
}
