use std::collections::HashMap;
use std::marker::PhantomData;
use std::net::{Ipv6Addr, SocketAddrV6};
use std::sync::Arc;
use std::time::Duration;

use crate::async_socket_options::AsyncSocketOptions;
use crate::error::{AsyncSocketResult, Status};
use crate::util::Empty;
use crate::uuid::Uuid;
use crate::EmptyEmitters;
use async_socket_internals::{DeMessage, SerMessage};
use futures_util::stream::SplitSink;
use futures_util::{future, SinkExt, StreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;

/// Provides the interface to initialize WebSocket calls to the client.
pub struct AsyncSocketContext<ServerEmitEvents = EmptyEmitters> {
  tx: UnboundedSender<ChannelMessage>,
  timeout: Duration,
  _p: PhantomData<ServerEmitEvents>,
}

impl<ServerEmitEvents: SerMessage> AsyncSocketContext<ServerEmitEvents> {
  fn new(tx: UnboundedSender<ChannelMessage>, timeout: Duration) -> Self {
    Self {
      tx,
      timeout,
      _p: PhantomData,
    }
  }

  /// Emit an event over the WebSocket. This should be used for events which do
  /// not expect a single return value from the receiver.
  pub fn emit(&mut self, event: ServerEmitEvents) -> AsyncSocketResult<()> {
    let msg: SocketMessage<ServerEmitEvents, Empty, Empty> = SocketMessage::Emit(event);
    let serialized =
      serde_json::to_string(&msg).map_err(|err| Status::SerializeFailed(err.to_string()))?;
    self
      .tx
      .send(ChannelMessage::SendMessage(Message::text(serialized)))
      .map_err(|err| Status::MessageQueueError(err.to_string()))?;

    Ok(())
  }

  /// Emit an event over the WebSocket which expects a response from the
  /// receiver. Returns an awaitable response (which may be an error), or
  /// `Status::Timeout` if the message timed out.
  pub async fn call<Req, Res>(&mut self, event: &Req) -> AsyncSocketResult<Res>
  where
    Req: SerMessage,
    Res: DeserializeOwned,
  {
    let uuid = Uuid::new_v4();

    let msg: SocketMessage<Empty, _, Empty> = SocketMessage::Call(CallMessage {
      uuid: uuid.clone(),
      client_call_events: event,
    });

    match serde_json::to_string(&msg) {
      Ok(s) => {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self
          .tx
          .send(ChannelMessage::QueueResponseListener {
            uuid: uuid.clone(),
            tx,
          })
          .map_err(|err| Status::MessageQueueError(err.to_string()))?;

        self
          .tx
          .send(ChannelMessage::SendMessage(Message::text(s)))
          .map_err(|err| {
            // If the SendMessage request fails, remove the queued response
            // listener.
            if self
              .tx
              .send(ChannelMessage::RemoveResponseListener { uuid: uuid.clone() })
              .is_err()
            {
              return Status::RemoveMessageQueueError(format!(
                "Failed to remove message {uuid} after send message request \
                failed to queue. Warning: this could cause a memory leak."
              ));
            }

            Status::MessageQueueError(err.to_string())
          })?;

        match tokio::time::timeout(self.timeout, rx).await {
          Ok(result) => {
            let response = result.map_err(|err| Status::RecvError(err.to_string()))?;

            match response {
              Status::Ok(response) => {
                let result: Res = serde_json::from_value(response)
                  .map_err(|err| Status::DeserializeFailed(err.to_string()))?;

                Ok(result)
              }
              _ => Err(response.clear_value()),
            }
          }
          Err(_) => {
            // If the SendMessage request fails, remove the queued response
            // listener.
            self
              .tx
              .send(ChannelMessage::RemoveResponseListener { uuid: uuid.clone() })
              .map_err(|_| {
                Status::RemoveMessageQueueError(format!(
                  "Failed to remove message {uuid} after timeout. Warning: \
                  this could cause a memory leak."
                ))
              })?;

            Err(Status::Timeout(format!(
              "Async call {uuid} timed out after {:?}",
              self.timeout
            )))
          }
        }
      }
      Err(error) => Err(Status::SerializeFailed(error.to_string())),
    }
  }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CallMessage<CallEvents> {
  uuid: Uuid,
  #[serde(flatten)]
  client_call_events: CallEvents,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseMessage<StatusResp> {
  uuid: Uuid,
  status: StatusResp,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum SocketMessage<EmitEvents, CallEvents, StatusResp> {
  Emit(EmitEvents),
  Call(CallMessage<CallEvents>),
  Response(ResponseMessage<StatusResp>),
}

struct AsyncSocketGlobals<ConnectEventFn, EmitEventFn, CallEventFn> {
  options: AsyncSocketOptions,
  connect_event_callback: ConnectEventFn,
  emit_event_callback: EmitEventFn,
  call_event_callback: CallEventFn,
}

enum ChannelMessage {
  SendMessage(Message),
  QueueResponseListener {
    uuid: Uuid,
    tx: tokio::sync::oneshot::Sender<Status<serde_json::Value>>,
  },
  RemoveResponseListener {
    uuid: Uuid,
  },
  TriggerResponse(ResponseMessage<Status<serde_json::Value>>),
}

/// Async Socket is an abstraction over WebSockets with typed messages and
/// call-response semantics.
pub struct AsyncSocket<
  ConnectEventFn,
  ClientEmitEvents,
  EmitEventFn,
  U,
  ServerEmitEvents,
  ClientCallEvents,
  CallEventFn,
  V,
  ClientResp,
> {
  globals: Arc<AsyncSocketGlobals<ConnectEventFn, EmitEventFn, CallEventFn>>,
  _p: PhantomData<(
    ClientEmitEvents,
    U,
    ServerEmitEvents,
    ClientCallEvents,
    V,
    ClientResp,
  )>,
}

impl<
    ConnectEventFn: Fn(AsyncSocketContext<ServerEmitEvents>) -> W + Send + Sync + 'static,
    W: std::future::Future<Output = ()> + Send + 'static,
    ClientEmitEvents: DeMessage + Send + Sync + 'static,
    EmitEventFn: Fn(ClientEmitEvents, AsyncSocketContext<ServerEmitEvents>) -> U + Send + Sync + 'static,
    U: std::future::Future<Output = ()> + Send + 'static,
    ServerEmitEvents: SerMessage + Send + 'static,
    ClientCallEvents: DeMessage + Send + Sync + 'static,
    CallEventFn: Fn(ClientCallEvents, AsyncSocketContext<ServerEmitEvents>) -> V + Send + Sync + 'static,
    V: std::future::Future<Output = Status<ClientResp>> + Send + 'static,
    ClientResp: SerMessage + 'static,
  >
  AsyncSocket<
    ConnectEventFn,
    ClientEmitEvents,
    EmitEventFn,
    U,
    ServerEmitEvents,
    ClientCallEvents,
    CallEventFn,
    V,
    ClientResp,
  >
{
  /// Constructs a new Async Socket manager.
  ///
  /// `connect_event_callback` is an async callback to call on every connection
  /// event to a client. It should take one parameter of type
  /// `AsyncSocketContext<ServerEmitEvents>`, where `ServerEmitEvents` is an
  /// enum of emit messages that can be made from this application to the
  /// client. `ServerEmitEvents` must derive `AsyncSocketEmitters`. The
  /// `AsyncSocketContext` is able to make `emit` and `call` calls to the
  /// client.
  ///
  /// `emit_event_callback` is an async callback to call for every emit event
  /// which is received from the client. It should take two parameters of types
  /// `ClientEmitEvents` and `AsyncSocketContext<ServerEmitEvents>`. The type
  /// `ClientEmitEvents` is an enum of emit messages that can be received from
  /// the client, and it must derive `AsyncSocketListeners`.
  ///
  /// `call_event_callback` is an async callback to call for every call event
  /// which is received from the client. It should take two parameters of types
  /// `ClientCallEvents` and `AsyncSocketContext<ServerEmitEvents>`. The type
  /// `ClientCallEvents` is an enum of call messages that can be received from
  /// the client, and it must derive `AsyncSocketListeners`. This callback must
  /// also return the response to each of these call messages, which must be a
  /// `Status<_>`. Idiomatically, the type returned is also an enum of messages
  /// that mirror the variants in `ClientCallEvents`, say `ServerRespEvents`.
  /// `ServerRespEvents` must derive `AsyncSocketResponders`.
  ///
  /// Example:
  /// ```
  /// #[derive(::async_sockets::AsyncSocketEmitters)]
  /// enum MyEmitEvents {
  ///   TestMessage { id: u64, name: String },
  /// }
  ///
  /// #[derive(::async_sockets::AsyncSocketEmitters)]
  /// enum MyCallEvents {
  ///   Test2 { id: u64 },
  /// }
  ///
  /// async fn handle_connect_event(mut context: ::async_sockets::AsyncSocketContext<MyEmitEvents>) {
  ///   context
  ///     .emit(MyEmitEvents::TestMessage {
  ///       id: 13,
  ///       name: "test".to_string(),
  ///     })
  ///     .unwrap();
  ///
  ///   let response: Result<u64, _> = context.call(&MyCallEvents::Test2 { id: 104 }).await;
  ///   match response {
  ///     Ok(response) => {
  ///       println!("Got response: {}", response);
  ///     }
  ///     Err(err) => {
  ///       println!("Error: {err}");
  ///     }
  ///   }
  /// }
  ///
  /// #[derive(::async_sockets::AsyncSocketListeners)]
  /// enum TheirEmitEvents {
  ///   TestMessage { id: u64 },
  /// }
  ///
  /// async fn handle_emit_event(
  ///   event: TheirEmitEvents,
  ///   _context: ::async_sockets::AsyncSocketContext<MyEmitEvents>,
  /// ) {
  ///   match event {
  ///     TheirEmitEvents::TestMessage { id } => {
  ///       println!("Got id {id}");
  ///     }
  ///   }
  /// }
  ///
  /// #[derive(::async_sockets::AsyncSocketListeners)]
  /// enum TheirCallEvents {
  ///   Test1 { id: u64 },
  /// }
  ///
  /// #[derive(::async_sockets::AsyncSocketResponders)]
  /// enum MyRespEvents {
  ///   Test1 { id: u64 },
  /// }
  ///
  /// async fn handle_call_event(
  ///   event: TheirCallEvents,
  ///   _context: ::async_sockets::AsyncSocketContext<MyEmitEvents>,
  /// ) -> ::async_sockets::Status<MyRespEvents> {
  ///   match event {
  ///     TheirCallEvents::Test1 { id } => {
  ///       ::async_sockets::Status::Ok(MyRespEvents::Test1 { id: id + 1 })
  ///     }
  ///   }
  /// }
  ///
  /// async fn run_server() {
  ///   let server = ::async_sockets::AsyncSocket::new(
  ///     ::async_sockets::AsyncSocketOptions::new()
  ///       .with_path("test")
  ///       .with_port(2345)
  ///       .with_timeout(::std::time::Duration::from_secs(10)),
  ///     handle_connect_event,
  ///     handle_emit_event,
  ///     handle_call_event,
  ///   );
  ///   server.start_server().await;
  /// }
  /// ```
  pub fn new(
    options: AsyncSocketOptions,
    connect_event_callback: ConnectEventFn,
    emit_event_callback: EmitEventFn,
    call_event_callback: CallEventFn,
  ) -> Self {
    Self {
      globals: Arc::new(AsyncSocketGlobals {
        options,
        connect_event_callback,
        emit_event_callback,
        call_event_callback,
      }),
      _p: PhantomData,
    }
  }

  /// Starts the Async Socket server, which will run the server forever on the
  /// current thread.
  pub async fn start_server(self) {
    let port = self.globals.options.port;

    let websocket = warp::path(self.globals.options.path.clone())
      .and(warp::ws())
      .then(move |ws: warp::ws::Ws| future::ready((ws, self.globals.clone())))
      .map(|(ws, globals): (warp::ws::Ws, _)| {
        ws.on_upgrade(|websocket| Self::handle_user_session(websocket, globals))
      });

    warp::serve(websocket)
      .run(SocketAddrV6::new(
        Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
        port,
        0,
        0,
      ))
      .await;
  }

  async fn process_global_messages(
    mut socket_tx: SplitSink<WebSocket, Message>,
    mut rx: UnboundedReceiverStream<ChannelMessage>,
  ) {
    struct OutstandingCall {
      tx: tokio::sync::oneshot::Sender<Status<serde_json::Value>>,
    }

    let mut outstanding_calls: HashMap<Uuid, OutstandingCall> = HashMap::new();

    while let Some(message) = rx.next().await {
      match message {
        ChannelMessage::SendMessage(message) => {
          socket_tx.send(message).await.unwrap_or_else(|err| {
            eprintln!("{err}");
          });
        }
        ChannelMessage::QueueResponseListener { uuid, tx } => {
          outstanding_calls.insert(uuid, OutstandingCall { tx });
        }
        ChannelMessage::RemoveResponseListener { uuid } => {
          outstanding_calls.remove(&uuid);
        }
        ChannelMessage::TriggerResponse(ResponseMessage { uuid, status }) => {
          if let Some(outstanding_call) = outstanding_calls.remove(&uuid) {
            if let Err(error) = outstanding_call.tx.send(status) {
              println!("Warning: receiver dropped after receiving async response: {error}");
            }
          } else {
            println!("Error: received response for unknown call {uuid}")
          }
        }
      }
    }
  }

  async fn handle_user_session(
    websocket: WebSocket,
    globals: Arc<AsyncSocketGlobals<ConnectEventFn, EmitEventFn, CallEventFn>>,
  ) {
    let (socket_tx, socket_rx) = websocket.split();
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);

    let async_socket_ctx = AsyncSocketContext::new(tx.clone(), globals.options.timeout);

    // Spawn a thread to handle user-defined connection response.
    tokio::task::spawn((globals.connect_event_callback)(async_socket_ctx));

    // Spawn a thread to continuously send all messages received on the channel.
    tokio::task::spawn(Self::process_global_messages(socket_tx, rx));

    socket_rx
      .take_while(|msg| {
        if let Ok(msg) = &msg {
          println!("Message: {msg:?}");
          if msg.is_close() {
            return future::ready(false);
          }
        }

        future::ready(true)
      })
      .filter_map(|msg| future::ready(msg.ok().filter(|msg| msg.is_text())))
      .then(|msg| future::ready((msg, tx.clone(), globals.clone())))
      .for_each(|(msg, tx, globals)| async move {
        tokio::task::spawn(async move {
          let result = Self::handle_message(msg.to_str().unwrap(), tx.clone(), globals).await;

          match result {
            Ok(Some(message)) => {
              if let Err(error) = tx.send(ChannelMessage::SendMessage(message)) {
                println!("Failed to send message over channel: {error}");
              }
            }
            Ok(None) => {}
            Err(error) => {
              eprintln!("{error}");
            }
          }
        });
      })
      .await;
  }

  async fn handle_message(
    message: &str,
    tx: UnboundedSender<ChannelMessage>,
    globals: Arc<AsyncSocketGlobals<ConnectEventFn, EmitEventFn, CallEventFn>>,
  ) -> Result<Option<Message>, Status> {
    let msg: Result<
      SocketMessage<ClientEmitEvents, ClientCallEvents, Status<serde_json::Value>>,
      _,
    > = serde_json::from_str(message);

    match msg {
      Ok(SocketMessage::Emit(client_emit_events)) => {
        (globals.emit_event_callback)(
          client_emit_events,
          AsyncSocketContext::new(tx, globals.options.timeout),
        )
        .await;
        Ok(None)
      }
      Ok(SocketMessage::Call(CallMessage::<ClientCallEvents> {
        uuid,
        client_call_events,
      })) => {
        println!("Call event: {uuid}");
        let status = (globals.call_event_callback)(
          client_call_events,
          AsyncSocketContext::new(tx, globals.options.timeout),
        )
        .await;

        let msg: SocketMessage<Empty, Empty, Status<ClientResp>> =
          SocketMessage::Response(ResponseMessage { uuid, status });

        println!("Responding with {}", serde_json::to_string(&msg).unwrap());
        let serialized =
          serde_json::to_string(&msg).map_err(|err| Status::SerializeFailed(err.to_string()))?;
        Ok(Some(Message::text(serialized)))
      }
      Ok(SocketMessage::Response(response)) => {
        println!("Response: {}", response.uuid);
        tx.send(ChannelMessage::TriggerResponse(response))
          .map_err(|err| {
            Status::MessageQueueError(format!(
              "Failed to send response message over oneshot::channel: {err}."
            ))
          })?;
        Ok(None)
      }
      Err(m) => Err(Status::DeserializeFailed(format!(
        "Unrecognized message format: {m}"
      ))),
    }
  }
}
