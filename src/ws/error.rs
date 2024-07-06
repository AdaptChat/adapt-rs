use tokio_tungstenite::tungstenite::protocol::CloseFrame;

/// A type alias for `Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// An error that occurs within the websocket module.
#[derive(Debug)]
pub enum Error {
    /// Unexpected message type received.
    UnexpectedMessageType,
    /// An error occured while connecting to the websocket.
    Connect(tokio_tungstenite::tungstenite::Error),
    /// An error occured while encoding a message using [`rmp_serde`].
    Encode(rmp_serde::encode::Error),
    /// An error occured while decoding a message using [`rmp_serde`].
    Decode(rmp_serde::decode::Error),
    /// The websocket connection was closed.
    Closed(Option<CloseFrame<'static>>),
    /// Expected a `hello` message from harmony, but received something else.
    NoHello,
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::Connect(err)
    }
}

impl From<rmp_serde::encode::Error> for Error {
    fn from(err: rmp_serde::encode::Error) -> Self {
        Self::Encode(err)
    }
}

impl From<rmp_serde::decode::Error> for Error {
    fn from(err: rmp_serde::decode::Error) -> Self {
        Self::Decode(err)
    }
}
