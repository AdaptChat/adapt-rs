//! A module for interacting with Harmony, Adapt's gateway.

pub mod error;
mod handler;

use essence::models::{Device, PresenceStatus};
use futures_util::{SinkExt, StreamExt};
use rmp_serde::to_vec_named;
use secrecy::{ExposeSecret, SecretString};
use std::{
    future::{Future, IntoFuture},
    time::{Duration, Instant},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
    MaybeTlsStream, WebSocketStream,
};
use url::Url;

use crate::Server;
pub use error::{Error, Result};
pub use essence::ws::{InboundMessage as OutboundMessage, OutboundMessage as InboundMessage};

#[allow(dead_code)] // TODO
enum WsAction {
    Reconnect,
}

/// A trait for types that can be converted into a valid URL for harmony.
pub trait IntoHarmonyUrl {
    /// Converts the type into a valid URL for harmony.
    fn into_harmony_url(self) -> Url;
}

impl IntoHarmonyUrl for Url {
    fn into_harmony_url(self) -> Url {
        self
    }
}

impl<'a> IntoHarmonyUrl for Server<'a> {
    fn into_harmony_url(self) -> Url {
        self.harmony.parse().unwrap()
    }
}

impl IntoHarmonyUrl for String {
    fn into_harmony_url(self) -> Url {
        self.parse().unwrap()
    }
}

/// Configuration options for connecting to the websocket.
///
/// # Example
/// ```no_run
/// use adapt::ws::{ConnectOptions, Result};
///
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// let token = std::env::var("ADAPT_TOKEN").expect("missing Adapt token");
/// let ws = ConnectOptions::new(token).await?;
/// # Ok(())
/// # }
#[derive(Clone, Debug)]
#[must_use = "This struct is a builder and should be used to create a `ws::Client` instance."]
pub struct ConnectOptions {
    /// The token to authenticate with.
    token: SecretString,
    /// The URL the client should connect to. Defaults to `wss://harmony.adapt.chat`.
    url: Url,
    /// The status to initially set the client's presence to.
    /// Defaults to [`PresenceStatus::Online`].
    status: PresenceStatus,
    /// The custom status to initially set the client's presence to. Defaults to `None`.
    custom_status: Option<String>,
    /// The device to identify as. Defaults to [`Device::Desktop`].
    device: Device,
}

impl ConnectOptions {
    /// Creates a new set of connect options with the default values.
    #[inline]
    pub fn new(token: impl AsRef<str>) -> Self {
        Self {
            token: SecretString::new(token.as_ref().to_string()),
            url: Server::production().into_harmony_url(),
            status: PresenceStatus::Online,
            custom_status: None,
            device: Device::Desktop,
        }
    }

    /// Sets the URL the client should connect to.
    #[inline]
    pub fn url(mut self, uri: impl IntoHarmonyUrl) -> Self {
        self.url = uri.into_harmony_url();
        self
    }

    /// Sets the status to initially set the client's presence to.
    #[inline]
    pub const fn status(mut self, status: PresenceStatus) -> Self {
        self.status = status;
        self
    }

    /// Sets the custom status to initially set the client's presence to.
    #[inline]
    pub fn custom_status(mut self, custom_status: Option<String>) -> Self {
        self.custom_status = custom_status;
        self
    }

    /// Sets the device to identify as.
    #[inline]
    pub const fn device(mut self, device: Device) -> Self {
        self.device = device;
        self
    }

    /// Connects to the websocket with these options.
    pub async fn connect(self) -> Result<Client> {
        Client::connect(self).await
    }
}

impl IntoFuture for ConnectOptions {
    type Output = Result<Client>;
    type IntoFuture = impl Future<Output = Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        self.connect()
    }
}

#[derive(Clone)]
struct PartialIdentify {
    status: PresenceStatus,
    custom_status: Option<String>,
    device: Device,
}

impl PartialIdentify {
    fn into_identify(self, token: &SecretString) -> OutboundMessage {
        OutboundMessage::Identify {
            token: token.expose_secret().clone(),
            status: self.status,
            custom_status: self.custom_status,
            device: self.device,
        }
    }
}

/// A client for interacting with harmony, Adapt's gateway.
pub struct Client {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    url: String,
    token: SecretString,
    identify: PartialIdentify,
    last_heartbeat_sent: Instant,
    latency: Option<Duration>,
}

impl Client {
    /// The timeout for receiving a message from the gateway.
    pub const TIMEOUT: Duration = Duration::from_millis(500);

    /// The interval at which the client should send heartbeats to the gateway.
    pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

    /// Initializes a new client and connects to the gateway.
    pub async fn connect(mut options: ConnectOptions) -> Result<Self> {
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
            url: options.url.to_string(),
            token: options.token,
            identify: PartialIdentify {
                status: options.status,
                custom_status: options.custom_status,
                device: options.device,
            },
            last_heartbeat_sent: Instant::now(),
            latency: None,
        })
    }

    async fn send(&mut self, value: &OutboundMessage) -> Result<()> {
        self.ws.send(Message::Binary(to_vec_named(value)?)).await?;

        Ok(())
    }

    /// Polls the websocket for the next message, or `None` if no messages can be received within
    /// [`Self::TIMEOUT`].
    pub async fn poll(&mut self) -> Result<Option<InboundMessage>> {
        let message = match tokio::time::timeout(Self::TIMEOUT, self.ws.next()).await {
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

    async fn handle_message(&mut self, message: &InboundMessage) -> Result<()> {
        match message {
            InboundMessage::Ping => {
                self.send(&OutboundMessage::Pong).await?;
                debug!("Acknowledged ping");
            }
            InboundMessage::Pong => {
                self.latency = Some(self.last_heartbeat_sent.elapsed());
                debug!("Heartbeat acknowledged, latency: {:?}", self.latency);
            }
            _ => {}
        }
        Ok(())
    }

    /// Runs the client's main loop for this session.
    pub async fn start(&mut self) -> Result<()> {
        if !matches!(self.poll().await?, Some(InboundMessage::Hello)) {
            return Err(Error::NoHello);
        }

        self.send_identify().await?;
        loop {
            // Send heartbeats at consistent intervals
            if self.last_heartbeat_sent.elapsed() >= Self::HEARTBEAT_INTERVAL {
                self.send_heartbeat().await?;
            }

            if let Some(message) = self.poll().await? {
                self.handle_message(&message).await?;
            }
        }
    }
}
