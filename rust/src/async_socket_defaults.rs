use async_socket_internals::{DeMessage, SerMessage};
use serde::{Deserialize, Serialize};

use crate::{error::Status, AsyncSocketContext};

#[derive(Serialize)]
pub enum EmptyEmitters {}

impl SerMessage for EmptyEmitters {}

#[derive(Deserialize)]
pub enum EmptyCallEvents {}

impl DeMessage for EmptyCallEvents {}

#[derive(Serialize)]
pub enum EmptyRespEvents {}

impl SerMessage for EmptyRespEvents {}

pub async fn ignore_connect_event<T>(_context: AsyncSocketContext<T>)
where
  T: SerMessage,
{
}

pub async fn no_emit_events<T>(event: EmptyEmitters, _context: AsyncSocketContext<T>)
where
  T: SerMessage,
{
  match event {}
}

pub async fn no_call_events<T>(
  event: EmptyCallEvents,
  _context: AsyncSocketContext<T>,
) -> Status<EmptyRespEvents>
where
  T: SerMessage,
{
  match event {}
}
