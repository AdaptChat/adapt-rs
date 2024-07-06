/// A collection of URLs used when interacting with the Adapt API.
#[derive(Copy, Clone, Debug)]
pub struct Server<'a> {
    /// The base URL for the REST API.
    pub api: &'a str,
    /// The base URL for harmony, Adapt's gateway (websocket).
    pub harmony: &'a str,
    /// The base URL for convey, Adapt's CDN.
    pub convey: &'a str,
}

impl Default for Server<'static> {
    fn default() -> Self {
        Self::production()
    }
}

impl Server<'static> {
    /// The official production Adapt instance. This is the default.
    pub const fn production() -> Self {
        Self {
            api: "https://api.adapt.chat",
            harmony: "wss://harmony.adapt.chat",
            convey: "https://convey.adapt.chat",
        }
    }

    /// A local instance of Adapt with default ports. Useful for self-hosted instances.
    pub const fn local() -> Self {
        Self {
            api: "http://localhost:8077",
            harmony: "ws://localhost:8076",
            convey: "http://localhost:8078",
        }
    }
}
