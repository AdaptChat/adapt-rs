pub mod error;

use std::time::{Duration, Instant};

use essence::{
    models::{Device, PresenceStatus},
    ws::{InboundMessage, OutboundMessage},
};
use futures_util::{SinkExt, StreamExt};
use rmp_serde::to_vec_named;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
    MaybeTlsStream, WebSocketStream,
};

use error::Result;
pub use error::WsError;

enum WsAction {
    Reconnect,
}

pub struct WsClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    url: String,
    token: String,
    status: PresenceStatus,
    device: Device,
    last_heartbeat_sent: Instant,
    last_heartbeat_acked: bool,
}

impl WsClient {
    pub async fn connect(
        url: String,
        token: String,
        status: PresenceStatus,
        device: Device,
    ) -> Result<Self> {
        let (stream, _) = connect_async_with_config(
            &url,
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
            url,
            token,
            status,
            device,
            last_heartbeat_sent: Instant::now(),
            last_heartbeat_acked: true,
        })
    }

    async fn send(&mut self, value: &impl Serialize) -> Result<()> {
        let message = to_vec_named(value)?;
        self.ws.send(Message::Binary(message)).await?;

        Ok(())
    }

    fn should_heartbeat(&self) -> bool {
        self.last_heartbeat_sent.elapsed() > Duration::from_secs(15)
    }

    async fn recv(&mut self) -> Result<Option<OutboundMessage>> {
        let message = match tokio::time::timeout(Duration::from_millis(800), self.ws.next()).await {
            Ok(Some(Ok(message))) => message,
            Ok(Some(Err(err))) => return Err(err.into()),
            Ok(None) | Err(_) => return Ok(None),
        };

        let decoded = match message {
            Message::Binary(bytes) => rmp_serde::from_slice(&bytes)?,
            Message::Text(_) => return Err(WsError::UnexpectedMessageType),
            Message::Close(frame) => return Err(WsError::WsClosed(frame)),
            _ => return Ok(None),
        };

        Ok(Some(decoded))
    }

    pub async fn start_runner(&mut self) -> Result<()> {
        if !matches!(self.recv().await?, Some(OutboundMessage::Hello)) {
            return Err(WsError::NoHello);
        }

        self.send(&InboundMessage::Identify {
            token: self.token.clone(),
            status: self.status,
            device: self.device,
        })
        .await?;

        todo!();

        Ok(())
    }
}
