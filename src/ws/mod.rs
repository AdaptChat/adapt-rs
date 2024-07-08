//! A module for interacting with Harmony, Adapt's gateway.

mod config;
mod connection;
pub mod error;
mod event;
pub mod handler;

use crate::Context;
use essence::models::{Device, PresenceStatus};
use handler::{EventConsumer, EventConsumerErased};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use tokio::sync::{
    mpsc::{channel, Sender},
    Mutex,
};

pub use config::{ConnectOptions, IntoHarmonyUrl};
pub use connection::Connection;
pub use error::{Error, Result};
pub use essence::ws::{InboundMessage as OutboundMessage, OutboundMessage as InboundMessage};
pub use event::Event;

#[derive(Clone)]
pub(super) struct PartialIdentify {
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

pub(crate) enum ClientAction {
    Reconnect,
    Close,
}

pub enum ConnectionAction {
    UpdatePresence {
        status: PresenceStatus,
        custom_status: Option<String>,
    },
    Close,
}

/// A cloneable messenger for interacting with an ongoing connection to the gateway.
#[derive(Clone)]
pub struct Messenger(Sender<ConnectionAction>);

impl Messenger {
    async fn send(&self, action: ConnectionAction) -> Result<()> {
        self.0.send(action).await?;
        Ok(())
    }

    /// Updates the presence of the client.
    pub async fn update_presence(
        &self,
        status: PresenceStatus,
        custom_status: Option<String>,
    ) -> Result<()> {
        self.send(ConnectionAction::UpdatePresence {
            status,
            custom_status,
        })
        .await?;
        Ok(())
    }

    /// Closes the connection to the gateway.
    pub async fn close(&self) -> Result<()> {
        self.send(ConnectionAction::Close).await?;
        Ok(())
    }
}

pub(super) type ConsumerVec = Arc<Mutex<Vec<Arc<dyn EventConsumerErased>>>>;

/// A client for interacting with harmony, Adapt's gateway.
#[derive(Clone)]
pub struct Client {
    /// Connect options to use when connecting to the gateway.
    options: ConnectOptions,
    /// The context template for models originating from this client.
    pub(crate) context: Context,
    /// Event consumers for incoming events.
    pub(crate) consumers: ConsumerVec,
}

impl Client {
    /// Creates a new client with the given connect options.
    #[must_use = "must call `start` to connect to the gateway"]
    pub fn new(options: ConnectOptions, context: Context) -> Self {
        Self {
            options,
            context,
            consumers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Registers an event consumer to receive incoming events.
    pub fn add_consumer(&self, consumer: impl EventConsumer + 'static) {
        self.consumers
            .try_lock()
            .expect("poison")
            .push(Arc::new(consumer));
    }

    /// Starts and maintains a connection to the gateway.
    pub async fn start(&mut self) -> Result<()> {
        let (client_tx, mut client_rx) = channel(1024);

        'a: loop {
            let (runner_tx, runner_rx) = channel(1024);
            let messenger = Messenger(runner_tx);
            self.context.ws = Some(messenger.clone());

            let mut connection = Connection::new(
                self.options.clone(),
                client_tx.clone(),
                runner_rx,
                self.consumers.clone(),
                self.context.clone(),
            )
            .await?;

            let tx = client_tx.clone();
            tokio::spawn(async move {
                if let Err(err) = connection.run().await {
                    warn!("Connection error: {:?}", err);
                    match err {
                        Error::Closed(_) => tx.send(ClientAction::Reconnect).await,
                        _ => tx.send(ClientAction::Close).await,
                    }
                    .ok();
                }
            });

            #[allow(clippy::never_loop)]
            while let Some(action) = client_rx.recv().await {
                match action {
                    ClientAction::Reconnect => {
                        messenger.close().await?;
                        continue 'a;
                    }
                    ClientAction::Close => {
                        messenger.close().await?;
                        break 'a;
                    }
                }
            }
        }

        self.context.ws = None;
        Ok(())
    }
}
