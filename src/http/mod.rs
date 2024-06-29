pub mod endpoints;

use crate::Error;
use bytes::Buf;
use endpoints::Endpoint;
use essence::http;
use reqwest::{
    header::{HeaderMap, HeaderName, AUTHORIZATION},
    Client,
};
use secrecy::{ExposeSecret, SecretString};
#[cfg(not(feature = "simd"))]
use serde_json as json;
#[cfg(feature = "simd")]
use simd_json as json;
use std::{
    future::{Future, IntoFuture},
    pin::Pin,
};

pub use http::auth::TokenRetrievalMethod;

/// A utility constant which is the base URL for the production (main) server of Adapt's API.
pub const BASE_URL: &str = AdaptServerUri::Production.as_str();

/// An enumeration of all pre-defined Adapt API base endpoints.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum AdaptServerUri<'a> {
    /// The production Adapt endpoint (`https://api.adapt.chat`). This is the default.
    #[default]
    Production,
    /// Localhost URI (`http://127.0.0.1:8077`) if you were to self-host Adapt.
    Local,
    /// Custom URI.
    Custom(&'a str),
}

impl<'a> AdaptServerUri<'a> {
    /// Returns the URI as a string.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'a str {
        match self {
            Self::Production => "https://api.adapt.chat",
            Self::Local => "http://127.0.0.1:8077",
            Self::Custom(uri) => uri,
        }
    }
}

impl<'a> From<&'a str> for AdaptServerUri<'a> {
    #[inline]
    fn from(uri: &'a str) -> Self {
        Self::Custom(uri)
    }
}

impl<'a> From<AdaptServerUri<'a>> for &'a str {
    #[inline]
    fn from(uri: AdaptServerUri<'a>) -> Self {
        uri.as_str()
    }
}

/// An outgoing HTTP request.
#[derive(Clone, Debug)]
#[must_use = "must .await the request to send it"]
pub struct Request<'a, E: Endpoint> {
    client: &'a Client,
    server: &'a str,
    endpoint: E,
    query: Option<E::Query>,
    body: Option<E::Body>,
    headers: HeaderMap,
}

impl<'a, E: Endpoint + 'a> IntoFuture for Request<'a, E> {
    type Output = crate::Result<E::Response>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

impl<'a, E: Endpoint> Request<'a, E> {
    /// Creates a new intermediate request.
    pub(super) fn new(client: &'a Client, server: &'a str, endpoint: E) -> Self {
        Self {
            client,
            server,
            endpoint,
            query: None,
            body: None,
            headers: HeaderMap::new(),
        }
    }

    /// Adds a header to the request.
    pub fn header(mut self, key: HeaderName, value: &str) -> Self {
        self.headers.insert(key, value.parse().unwrap());
        self
    }

    /// Adds query parameters to the request.
    pub fn query(mut self, query: E::Query) -> Self {
        self.query = Some(query);
        self
    }

    /// Sets the body of the request.
    pub fn body(mut self, body: E::Body) -> Self {
        self.body = Some(body);
        self
    }

    /// Sends the request.
    pub async fn send(self) -> crate::Result<E::Response> {
        let mut request = self
            .client
            .request(E::METHOD, self.server.to_string() + &self.endpoint.path())
            .headers(self.headers);

        if let Some(query) = self.query {
            request = request.query(&query);
        }

        if let Some(body) = self.body {
            let body = json::to_string(&body).unwrap();

            request = request
                .body(body)
                .header("Content-Type", "application/json");
        }

        let response = request.send().await?;
        let status = response.status().as_u16();
        let reader = response.bytes().await?.reader();

        if (400..=599).contains(&status) {
            let error = json::from_reader(reader)?;

            return Err(Error::Adapt(error));
        }

        let data = json::from_reader(reader);

        data.map_err(Into::into)
    }
}

/// The underlying HTTP client for the Adapt REST API.
///
/// # Example
/// ```no_run
/// use adapt::essence::http::message::CreateMessagePayload;
/// use adapt::http::{Http, endpoints};
///
/// #[tokio::main]
/// async fn main() -> adapt::Result<()> {
///     let token = std::env::var("ADAPT_TOKEN").expect("missing Adapt token");
///     let http = Http::from_token(token);
///
///     let payload = CreateMessagePayload {
///         content: Some("Hello, world!".to_string()),
///        ..Default::default()
///     };
///     let message = http.request(endpoints::CreateMessage(123456789)).body(payload).await?;
///     println!("Created message: {}", message.content.unwrap());
///     Ok(())
/// }
#[derive(Clone, Debug)]
pub struct Http {
    client: Client,
    server: String,
    token: SecretString,
}

impl Http {
    /// Creates a new HTTP client with the given token and Adapt server URI.
    ///
    /// # Example
    /// ```no_run
    /// # use adapt::http::{AdaptServerUri, Http};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> adapt::Result<()> {
    /// let token = std::env::var("ADAPT_TOKEN").expect("missing Adapt token");
    /// let http = Http::from_token_and_uri(token, AdaptServerUri::Production);
    /// # Ok(()) }
    /// ```
    ///
    /// # Panics
    /// * If an error occurs while creating the client.
    /// * If the token is not a valid header value.
    #[must_use]
    pub fn from_token_and_uri(token: impl AsRef<str>, uri: AdaptServerUri) -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .expect("failed to initialize HTTP client");

        Self {
            client,
            server: uri.as_str().to_string(),
            token: SecretString::new(token.as_ref().to_string()),
        }
    }

    /// Creates a new HTTP client with the given token and the default Adapt server URI.
    /// See [`AdaptServerUri`] for more information of what this is.
    ///
    /// # Panics
    /// * If an error occurs while creating the client.
    /// * If the token is not a valid header value.
    #[must_use]
    pub fn from_token(token: impl AsRef<str>) -> Self {
        Self::from_token_and_uri(token, AdaptServerUri::Production)
    }

    /// Logs into the given user account with credentials (email and password) and creates
    /// a new HTTP client for that user. The Adapt API will return a token based on the given
    /// token retrieval method (specified by the `retrieval_method` parameter); when in doubt,
    /// use `Default::default()`.
    ///
    /// The Adapt server URI will be determined by the `server` parameter. Returns a new HTTP
    /// client for that user on success.
    ///
    /// # See also
    /// * [`TokenRetrievalMethod`] which is used to determine how the token is retrieved.
    /// * [`Self::login`] which is a convenience method for logging in with the production server.
    pub async fn login_on(
        server: AdaptServerUri<'_>,
        email: impl AsRef<str> + Send,
        password: impl AsRef<str> + Send,
        retrieval_method: TokenRetrievalMethod,
    ) -> crate::Result<Self> {
        let mut slf = Self::from_token_and_uri("", server);
        let user = slf
            .request(endpoints::Login)
            .body(http::auth::LoginRequest {
                email: email.as_ref().to_string(),
                password: password.as_ref().to_string(),
                method: retrieval_method,
            })
            .await?;

        slf.token = SecretString::new(user.token);
        Ok(slf)
    }

    /// Logs into the given user account with credentials (email and password) on the production
    /// server and creates a new HTTP client for that user. The Adapt API will return a token
    /// based on the given token retrieval method (specified by the `retrieval_method` parameter);
    /// when in doubt, use `Default::default()`.
    ///
    /// Returns a new HTTP client for that user on success.
    ///
    /// # Example
    /// ```no_run
    /// use adapt::http::{AdaptServerUri, Http, TokenRetrievalMethod, endpoints::GetAllGuilds};
    ///
    /// #[tokio::main]
    /// async fn main() -> adapt::Result<()> {
    ///     let http = Http::login(
    ///         "user@example.com",
    ///         "password",
    ///         TokenRetrievalMethod::Reuse,
    ///     )
    ///     .await?;
    ///
    ///     // do stuff with the HTTP client
    ///     let guilds = http.request(GetAllGuilds).await?;
    ///     println!("You are in {} guilds", guilds.len());
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn login(
        email: impl AsRef<str> + Send,
        password: impl AsRef<str> + Send,
        retrieval_method: TokenRetrievalMethod,
    ) -> crate::Result<Self> {
        Self::login_on(
            AdaptServerUri::Production,
            email,
            password,
            retrieval_method,
        )
        .await
    }

    /// Returns the authentication token for this client. You should not expose this value to
    /// anyone.
    #[inline]
    #[must_use]
    pub const fn token(&self) -> &SecretString {
        &self.token
    }

    /// Creates a new outgoing HTTP request to the given endpoint. The request takes and returns raw
    /// models from [`essence`].
    pub fn request<E: Endpoint>(&self, endpoint: E) -> Request<E> {
        let token = self.token.expose_secret();
        Request::new(&self.client, &self.server, endpoint).header(AUTHORIZATION, token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::println;

    #[tokio::test]
    async fn get_channel() -> crate::Result<()> {
        let token = std::env::var("ADAPT_TOKEN").expect("missing Adapt token");
        let http = Http::from_token(token);

        println!("{:#?}", http.request(endpoints::GetAuthenticatedUser).await);
        Ok(())
    }
}
