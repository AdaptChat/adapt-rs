use super::{
    ClientAction, ConnectOptions, ConnectionAction, Consumer, Error, InboundMessage,
    OutboundMessage, PartialIdentify, Result,
};
use crate::ws::event::populate;
use crate::Context;
use essence::models::PresenceStatus;
use futures_util::{SinkExt, StreamExt};
use rmp_serde::to_vec_named;
use secrecy::SecretString;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tokio::{
    net::TcpStream,
    sync::mpsc::{Receiver, Sender},
};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
    MaybeTlsStream, WebSocketStream,
};

/// Manages a single connection to Harmony.
///
/// A connection is
pub struct Connection {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    token: SecretString,
    identify: PartialIdentify,
    last_heartbeat_sent: Instant,
    latency: Option<Duration>,
    #[allow(dead_code)]
    client_tx: Sender<ClientAction>,
    runner_rx: Receiver<ConnectionAction>,
    consumer: Consumer,
    context: Context,
}

impl Connection {
    /// The timeout for receiving a message from the gateway. If no message is received within this
    /// duration, the client will attempt to reconnect.
    pub const TIMEOUT: Duration = Duration::from_millis(500);

    /// The interval at which the client should send heartbeats to the gateway.
    pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

    /// The timeout for acquiring a lock to the event consumers. If the lock cannot be acquired
    /// within this duration, the event will be ignored.
    pub const ACQUIRE_TIMEOUT: Duration = Duration::from_millis(500);

    /// Initializes a new client and connects to the gateway.
    pub(crate) async fn new(
        mut options: ConnectOptions,
        client_tx: Sender<ClientAction>,
        runner_rx: Receiver<ConnectionAction>,
        consumer: Consumer,
        context: Context,
    ) -> Result<Self> {
        options.url.set_query(Some("format=msgpack"));
        let (stream, _) = connect_async_with_config(
            options.url.as_str(),
            Some(WebSocketConfig {
                max_message_size: None,
                max_frame_size: None,
                ..Default::default()
            }),
            false,
        )
        .await?;

        Ok(Self {
            ws: stream,
            token: options.token,
            identify: PartialIdentify {
                status: options.status,
                custom_status: options.custom_status,
                device: options.device,
            },
            last_heartbeat_sent: Instant::now(),
            latency: None,
            client_tx,
            runner_rx,
            consumer,
            context,
        })
    }

    async fn send(&mut self, value: &OutboundMessage) -> Result<()> {
        self.ws.send(Message::Binary(to_vec_named(value)?)).await?;

        Ok(())
    }

    /// Polls the websocket for the next message, or `None` if no messages can be received within
    /// [`Self::TIMEOUT`].
    pub async fn poll(&mut self) -> Result<Option<InboundMessage>> {
        let message = match timeout(Self::TIMEOUT, self.ws.next()).await {
            Ok(Some(Ok(message))) => message,
            Ok(Some(Err(err))) => return Err(err.into()),
            Ok(None) | Err(_) => return Ok(None),
        };

        let decoded = match message {
            Message::Binary(bytes) => rmp_serde::from_slice(&bytes)?,
            Message::Text(_) => return Err(Error::UnexpectedMessageType),
            Message::Close(frame) => return Err(Error::Closed(frame)),
            _ => return Ok(None),
        };

        Ok(Some(decoded))
    }

    /// Sends an identify message to the gateway.
    pub async fn send_identify(&mut self) -> Result<()> {
        debug!("Sending identify");
        let identify = self.identify.clone().into_identify(&self.token);
        self.send(&identify).await
    }

    /// Sends a heartbeat to the gateway.
    pub async fn send_heartbeat(&mut self) -> Result<()> {
        debug!("Sending heartbeat");
        self.send(&OutboundMessage::Ping).await?;
        self.last_heartbeat_sent = Instant::now();
        Ok(())
    }

    /// Sends a presence update request to the gateway.
    pub async fn send_update_presence(
        &mut self,
        status: PresenceStatus,
        custom_status: Option<String>,
    ) -> Result<()> {
        let payload = OutboundMessage::UpdatePresence {
            status,
            custom_status,
        };
        self.send(&payload).await
    }

    async fn handle_message(&mut self, message: InboundMessage) -> Result<()> {
        match message {
            InboundMessage::Ping => {
                self.send(&OutboundMessage::Pong).await?;
                debug!("Acknowledged ping");
            }
            InboundMessage::Pong => {
                self.latency = Some(self.last_heartbeat_sent.elapsed());
                debug!("Heartbeat acknowledged, latency: {:?}", self.latency);
            }
            event => {
                let mut events = Vec::with_capacity(4);
                populate(self.context.clone(), event, &mut events);

                if !events.is_empty() {
                    debug!("Attempting to dispatch event");
                    let consumers = timeout(Self::ACQUIRE_TIMEOUT, self.consumer.lock()).await;
                    if let Ok(mut consumers) = consumers {
                        for event in events {
                            consumers.dyn_handle_event(event).await;
                        }
                    } else {
                        warn!("Could not acquire lock to dispatch event");
                    }
                }
            }
        }
        Ok(())
    }

    /// Runs the main loop for this session.
    pub async fn run(&mut self) -> Result<()> {
        if !matches!(self.poll().await?, Some(InboundMessage::Hello)) {
            return Err(Error::NoHello);
        }

        self.send_identify().await?;
        loop {
            // Send heartbeats at consistent intervals
            if self.last_heartbeat_sent.elapsed() >= Self::HEARTBEAT_INTERVAL {
                self.send_heartbeat().await?;
            }

            if let Ok(action) = self.runner_rx.try_recv() {
                match action {
                    ConnectionAction::UpdatePresence {
                        status,
                        custom_status,
                    } => {
                        self.send_update_presence(status, custom_status).await?;
                    }
                    ConnectionAction::Close => {
                        debug!("Received close action, shutting down connection...");
                        self.ws.close(None).await?;
                        return Ok(());
                    }
                }
            }

            if let Some(message) = self.poll().await? {
                self.handle_message(message).await?;
            }
        }
    }
}
