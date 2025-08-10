//! Interact with Adapt using the client module.

mod context;

#[cfg(feature = "ws")]
use crate::ws;
use crate::{http::Http, Result, Server};
use essence::models::{Device, PresenceStatus};
use std::sync::Arc;
#[cfg(feature = "ws")]
use tokio::sync::Mutex;

pub use context::{Context, WithCtx};

/// Configures options for a [`Client`].
#[derive(Clone)]
#[must_use = "must `.build()` a `Client` to connect to Adapt"]
pub struct ClientOptions<'a> {
    /// The token to use for authentication.
    pub token: String,
    /// The server where Adapt is hosted.
    pub server: Server<'a>,
    /// The options for connecting to the gateway.
    #[cfg(feature = "ws")]
    pub ws_options: ws::ConnectOptions,
    /// The event consumer for handling events from the gateway.
    #[cfg(feature = "ws")]
    pub(crate) ws_consumer: ws::Consumer,
}

impl<'a> ClientOptions<'a> {
    /// Creates a new set of client options with the given token and server.
    pub fn from_server(token: impl AsRef<str>, server: Server<'a>) -> Self {
        Self {
            token: token.as_ref().to_string(),
            server,
            #[cfg(feature = "ws")]
            ws_options: ws::ConnectOptions::new(token),
            #[cfg(feature = "ws")]
            ws_consumer: Arc::new(Mutex::new(ws::handler::from_fn(|_| async {}))),
        }
    }

    /// Sets the status to initially set the client's presence to.
    #[inline]
    pub fn status(mut self, status: PresenceStatus) -> Self {
        self.ws_options = self.ws_options.status(status);
        self
    }

    /// Sets the custom status to initially set the client's presence to.
    #[inline]
    pub fn custom_status(mut self, custom_status: impl AsRef<str>) -> Self {
        self.ws_options = self
            .ws_options
            .custom_status(Some(custom_status.as_ref().to_string()));
        self
    }

    /// Sets the device to identify as.
    #[inline]
    pub fn device(mut self, device: Device) -> Self {
        self.ws_options = self.ws_options.device(device);
        self
    }

    /// Sets the event consumer, which will receive events from the gateway.
    ///
    /// # Example
    /// ```no_run
    /// use adapt::ws::handler;
    ///
    /// let client = adapt::ClientOptions::new("token")
    ///     .consumer(handler::from_fn(|event| async {
    ///         println!("Received event: {:?}", event);
    ///     }))
    ///     .into_client();
    /// ```
    #[cfg(feature = "ws")]
    pub fn consumer(mut self, consumer: impl ws::EventConsumer + 'static) -> Self {
        self.ws_consumer = Arc::new(Mutex::new(consumer));
        self
    }

    /// Builds a new [`Client`] with these options.
    pub fn into_client(self) -> Client {
        Client::from_options(self)
    }
}

impl ClientOptions<'static> {
    /// Creates a new set of client options which uses the given token.
    pub fn new(token: impl AsRef<str>) -> Self {
        Self::from_server(token, Server::default())
    }
}

/// Allows interaction with the Adapt API by unifying the following:
///
/// - Access to the REST API (see [`Http`])
/// - Access to the gateway connection (see [`Messenger`][crate::ws::Messenger])
/// - Resolution and caching of models
///
/// Typically, this client is only used to initialize and connect to Adapt, whereafter [`Context`]
/// is used as a shared state for interacting with the API.
#[must_use = "must call `start` to connect to the gateway"]
pub struct Client {
    /// The HTTP client used to make requests to the REST API.
    pub http: Arc<Http>,
    /// The websocket client maintaing connections with the gateway.
    #[cfg(feature = "ws")]
    pub ws: ws::Client,
}

impl Client {
    /// Creates a new client using a token.
    pub fn from_token(token: impl AsRef<str>) -> Self {
        Self::from_options(ClientOptions::new(token))
    }

    /// Creates a new client with the given options.
    pub fn from_options(options: ClientOptions) -> Self {
        let http = Http::from_token_and_uri(&options.token, options.server);

        #[cfg(feature = "ws")]
        let ws = ws::Client::from_wrapped_consumer(options.ws_options, options.ws_consumer);

        Self {
            http: Arc::new(http),
            #[cfg(feature = "ws")]
            ws,
        }
    }

    /// Sets the event consumer for the client.
    #[cfg(feature = "ws")]
    pub fn set_consumer(&mut self, consumer: impl ws::EventConsumer + 'static) {
        self.ws.consumer = Arc::new(Mutex::new(consumer));
    }

    /// Sets the client's event consumer to the given consumer, and then returns the client.
    #[cfg(feature = "ws")]
    pub fn with_consumer(mut self, consumer: impl ws::EventConsumer + 'static) -> Self {
        self.set_consumer(consumer);
        self
    }

    /// Starts the client, connecting to the gateway and initializing the cache.
    pub async fn start(&self) -> Result<Context> {
        let ctx = Context {
            http: self.http.clone(),
            #[cfg(feature = "ws")]
            ws: None,
        };

        #[cfg(feature = "ws")]
        self.ws.start(ctx.clone()).await?;

        Ok(ctx)
    }
}
