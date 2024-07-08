use crate::Server;
use essence::models::{Device, PresenceStatus};
use secrecy::SecretString;
use url::Url;

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
#[derive(Clone, Debug)]
#[must_use = "This struct is a builder and should be used to create a `ws::Client` instance."]
pub struct ConnectOptions {
    /// The token to authenticate with.
    pub(crate) token: SecretString,
    /// The URL the client should connect to. Defaults to `wss://harmony.adapt.chat`.
    pub(crate) url: Url,
    /// The status to initially set the client's presence to.
    /// Defaults to [`PresenceStatus::Online`].
    pub status: PresenceStatus,
    /// The custom status to initially set the client's presence to. Defaults to `None`.
    pub custom_status: Option<String>,
    /// The device to identify as. Defaults to [`Device::Desktop`].
    pub device: Device,
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
}
