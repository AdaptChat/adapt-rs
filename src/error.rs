pub type Result<T> = std::result::Result<T, Error>;

/// An error that occurs within the crate.
#[derive(Debug)]
pub enum Error {
    /// An error occured within reqwest while requesting a resource from the Adapt API.
    Reqwest(reqwest::Error),
    /// An error occured while deserializing a response from the Adapt API.
    #[cfg(feature = "simd-json")]
    Deserialization(simd_json::Error),
    #[cfg(not(feature = "simd-json"))]
    Deserialization(serde_json::Error),
    /// An error was returned from the Adapt.
    Adapt(essence::Error),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Reqwest(err)
    }
}

#[cfg(not(feature = "simd-json"))]
impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Deserialization(err)
    }
}

#[cfg(feature = "simd-json")]
impl From<simd_json::Error> for Error {
    fn from(err: simd_json::Error) -> Self {
        Self::Deserialization(err)
    }
}
