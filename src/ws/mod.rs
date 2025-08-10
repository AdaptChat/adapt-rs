//! A module for interacting with Harmony, Adapt's gateway.

mod config;
mod connection;
pub mod error;
mod event;
pub mod handler;

use crate::Context;
use essence::models::{Device, PresenceStatus};
use handler::EventConsumerErased;
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
pub use handler::{EventConsumer, EventHandler, FallibleEventHandler};

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

pub(super) type Consumer = Arc<Mutex<dyn EventConsumerErased>>;

/// A client for interacting with harmony, Adapt's gateway.
#[derive(Clone)]
pub struct Client {
    /// Connect options to use when connecting to the gateway.
    options: ConnectOptions,
    /// Event consumer for incoming events.
    pub(crate) consumer: Consumer,
}

impl Client {
    /// Creates a new client with the given connect options.
    #[must_use = "must call `start` to connect to the gateway"]
    pub fn new(options: ConnectOptions, consumer: impl EventConsumer + 'static) -> Self {
        Self::from_wrapped_consumer(options, Arc::new(Mutex::new(consumer)))
    }

    pub(crate) fn from_wrapped_consumer(options: ConnectOptions, consumer: Consumer) -> Self {
        Self { options, consumer }
    }

    /// Starts and maintains a connection to the gateway.
    pub async fn start(&self, mut context: Context) -> Result<()> {
        let (client_tx, mut client_rx) = channel(1024);

        'a: loop {
            let (runner_tx, runner_rx) = channel(1024);
            let messenger = Messenger(runner_tx);
            context.ws = Some(messenger.clone());

            let mut connection = Connection::new(
                self.options.clone(),
                client_tx.clone(),
                runner_rx,
                self.consumer.clone(),
                context.clone(),
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

        context.ws = None;
        Ok(())
    }
}
