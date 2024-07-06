pub mod error;

use essence::{
    models::{Device, PresenceStatus},
    ws::{InboundMessage, OutboundMessage},
};
use futures_util::{SinkExt, StreamExt};
use rmp_serde::to_vec_named;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use std::future::{Future, IntoFuture};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
    MaybeTlsStream, WebSocketStream,
};
use url::Url;

use crate::Server;
pub use error::{Error, Result};

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
    pub fn status(mut self, status: PresenceStatus) -> Self {
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
    pub fn device(mut self, device: Device) -> Self {
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
    fn into_identify(self, token: &SecretString) -> InboundMessage {
        InboundMessage::Identify {
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
    pub const TIMEOUT: Duration = Duration::from_millis(800);

    /// The interval at which the client should send heartbeats to the gateway.
    ///
    /// # Note
    /// Currently, since Harmony does not require heartbeats to be sent at a consistent interval,
    /// this value is actually the lowest interval that can pass from the last heartbeat before the
    /// client sends another one.
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

    async fn send(&mut self, value: &impl Serialize) -> Result<()> {
        self.ws.send(Message::Binary(to_vec_named(value)?)).await?;

        Ok(())
    }

    async fn next(&mut self) -> Result<Option<OutboundMessage>> {
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

    pub async fn send_identify(&mut self) -> Result<()> {
        let identify = self.identify.clone().into_identify(&self.token);
        self.send(&identify).await
    }

    async fn start(&mut self) -> Result<()> {
        tokio::select! {
            _ = self.keep_alive() => { Ok(()) }
            _ = self.runner() => { Ok(()) }
        }
    }

    async fn keep_alive(&mut self) -> Result<()> {
        loop {
            tokio::time::sleep(Self::HEARTBEAT_INTERVAL).await;
            self.send(&InboundMessage::Ping).await?;
            self.last_heartbeat_sent = Instant::now();
        }
    }

    pub async fn runner(&mut self) -> Result<()> {
        if !matches!(self.next().await?, Some(OutboundMessage::Hello)) {
            return Err(Error::NoHello);
        }

        self.send_identify().await?;
        while let Some(message) = self.next().await? {
            match message {
                OutboundMessage::Ping => {
                    self.send(&InboundMessage::Pong).await?;
                }
                OutboundMessage::Pong => {
                    self.latency = Some(self.last_heartbeat_sent.elapsed());
                }
                _ => {}
            }
        }

        todo!();

        Ok(())
    }
}
