pub mod endpoints;

use crate::Error;
use bytes::Buf;
use endpoints::Endpoint;
use essence::{
    http::{
        auth::{LoginRequest, LoginResponse, TokenRetrievalMethod},
        channel::{CreateGuildChannelPayload, EditChannelPayload},
        guild::{CreateGuildPayload, DeleteGuildPayload, EditGuildPayload},
        role::{CreateRolePayload, EditRolePayload},
        user::{CreateUserPayload, CreateUserResponse, DeleteUserPayload, EditUserPayload},
    },
    models as raw,
};
use reqwest::{
    header::{HeaderMap, HeaderName, AUTHORIZATION},
    Client,
};
#[cfg(feature = "simd")]
use simd_json as json;
#[cfg(not(feature = "simd"))]
use serde_json as json;
use serde::{de::DeserializeOwned, ser::Serialize};

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
pub struct Request<'a, E: Endpoint, S: Serialize> {
    client: &'a Client,
    server: &'a str,
    endpoint: E,
    body: Option<S>,
    headers: HeaderMap,
}

impl<'a, E: Endpoint, S: Serialize> Request<'a, E, S> {
    /// Creates a new intermediate request.
    pub(super) fn new(client: &'a Client, server: &'a str, endpoint: E, body: Option<S>) -> Self {
        Self {
            client,
            server,
            endpoint,
            body,
            headers: HeaderMap::new(),
        }
    }

    /// Adds a header to the request.
    pub fn header(mut self, key: HeaderName, value: &str) -> Self {
        self.headers.insert(key, value.parse().unwrap());
        self
    }

    /// Sends the request.
    pub async fn send<T: DeserializeOwned>(self) -> crate::Result<T> {
        let mut request = self
            .client
            .request(E::METHOD, self.server.to_string() + &self.endpoint.path())
            .headers(self.headers);

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

/// An HTTP client for Adapt's REST API.
pub struct Http {
    client: Client,
    server: String,
    token: String,
}

macro_rules! http_methods {
    (
        @one
        $(#[$doc:meta])*
        $name:ident($($params:ident: $param_ty:ty),*) -> $res:ty = $endpoint:ident
    ) => {
        $(#[$doc])*
        pub async fn $name(&self, $($params: $param_ty),*) -> crate::Result<$res> {
            self.request(endpoints::$endpoint $(.$params($params))*).send().await
        }
    };
    (
        @one
        $(#[$doc:meta])*
        $name:ident($($params:ident: $param_ty:ty),*) -> $res:ty = $endpoint:ident
        ($($body:ident: $body_ty:ty),+ $(,)?) => $mk_body:expr
    ) => {
        $(#[$doc])*
        pub async fn $name(&self, $($params: $param_ty,)* $($body: $body_ty),+) -> crate::Result<$res> {
            self.request_with_body(endpoints::$endpoint $(.$params($params))*, $mk_body)
                .send()
                .await
        }
    };
    (
        $(
            $(#[$doc:meta])*
            $name:ident($($params:ident: $param_ty:ty),*) -> $res:ty = $endpoint:ident
            $(($($body:ident: $body_ty:ty),+ $(,)?) => $mk_body:expr)?
        ),+ $(,)?
    ) => {
        $(
            http_methods!(
                @one $(#[$doc])*
                $name($($params: $param_ty),*) -> $res = $endpoint
                $(($($body: $body_ty),+) => $mk_body)?
            );
        )+
    };
}

impl Http {
    /// Creates a new HTTP client with the given token and Adapt server URI.
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
            token: token.as_ref().to_string(),
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

    /// Creates a new user account with the given username, email, and password, and creates
    /// a new HTTP client for that user. The Adapt server URI will be determined by the `server`
    /// parameter. Returns a new HTTP client for that user on success.
    pub async fn create_user_on(
        server: AdaptServerUri<'_>,
        username: impl AsRef<str>,
        email: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> crate::Result<Self> {
        let mut slf = Self::from_token_and_uri("", server);
        let user = slf
            .request_with_body(
                endpoints::CREATE_USER,
                CreateUserPayload {
                    username: username.as_ref().to_string(),
                    email: email.as_ref().to_string(),
                    password: password.as_ref().to_string(),
                },
            )
            .send::<CreateUserResponse>()
            .await?;

        slf.token = user.token;
        Ok(slf)
    }

    /// Creates a new user account with the given user name, email, and password on the production
    /// server. Returns a new HTTP client for that user on success.
    pub async fn create_user(
        username: impl AsRef<str>,
        email: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> crate::Result<Self> {
        Self::create_user_on(AdaptServerUri::Production, username, email, password).await
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
        email: impl AsRef<str>,
        password: impl AsRef<str>,
        retrieval_method: TokenRetrievalMethod,
    ) -> crate::Result<Self> {
        let mut slf = Self::from_token_and_uri("", server);
        let user = slf
            .request_with_body(
                endpoints::LOGIN,
                LoginRequest {
                    email: email.as_ref().to_string(),
                    password: password.as_ref().to_string(),
                    method: retrieval_method,
                },
            )
            .send::<LoginResponse>()
            .await?;

        slf.token = user.token;
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
    /// use adapt_chat::essence::http::auth::TokenRetrievalMethod;
    /// use adapt_chat::http::{AdaptServerUri, Http};
    ///
    /// #[tokio::main]
    /// async fn main() -> adapt_chat::Result<()> {
    ///     let http = Http::login(
    ///         "user@example.com",
    ///         "password",
    ///         TokenRetrievalMethod::Reuse,
    ///     )
    ///     .await?;
    ///
    ///     // do stuff with the HTTP client
    ///     let guilds = http.get_all_guilds().await?;
    ///     println!("You are in {} guilds", guilds.len());
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn login(
        email: impl AsRef<str>,
        password: impl AsRef<str>,
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
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Creates a new outgoing HTTP request to the given endpoint.
    pub fn request<E: Endpoint>(&self, endpoint: E) -> Request<E, ()> {
        Request::new(&self.client, &self.server, endpoint, None).header(AUTHORIZATION, &self.token)
    }

    /// Creates a new outgoing HTTP request to the given endpoint with the given body.
    pub fn request_with_body<E: Endpoint, S: Serialize>(
        &self,
        endpoint: E,
        body: S,
    ) -> Request<E, S> {
        Request::new(&self.client, &self.server, endpoint, Some(body))
            .header(AUTHORIZATION, &self.token)
    }

    http_methods! {
        /// Fetches a channel by its ID.
        get_channel(channel_id: u64) -> raw::Channel = GET_CHANNEL,
        /// Deletes a channel by its ID.
        delete_channel(channel_id: u64) -> () = DELETE_CHANNEL,
        /// Edits a channel by its ID.
        edit_channel(channel_id: u64) -> raw::Channel = EDIT_CHANNEL(payload: EditChannelPayload) => {
            payload
        },
        /// Fetches all channels in the given guild.
        get_guild_channels(guild_id: u64) -> Vec<raw::Channel> = GET_GUILD_CHANNELS,
        /// Creates a new channel in the given guild.
        create_guild_channel(guild_id: u64) -> raw::Channel = CREATE_GUILD_CHANNEL(
            payload: CreateGuildChannelPayload,
        ) => payload,
        /// Fetches all guilds assigned to the current user.
        get_all_guilds() -> Vec<raw::Guild> = GET_ALL_GUILDS,
        /// Creates a guild with the given payload.
        create_guild() -> raw::Guild = CREATE_GUILD(payload: CreateGuildPayload) => payload,
        /// Fetches a guild by its ID.
        get_guild(guild_id: u64) -> raw::Guild = GET_GUILD,
        /// Edits a guild by its ID.
        edit_guild(guild_id: u64) -> raw::Guild = EDIT_GUILD(payload: EditGuildPayload) => payload,
        /// Deletes a guild by its ID.
        delete_guild(guild_id: u64) -> () = DELETE_GUILD(password: Option<impl AsRef<str>>) => {
            password.map(|password| DeleteGuildPayload {
                password: password.as_ref().to_string(),
            })
        },
        /// Gets all roles in the given guild.
        get_guild_roles(guild_id: u64) -> Vec<raw::Role> = GET_GUILD_ROLES,
        /// Creates a new role in the given guild.
        create_role(guild_id: u64) -> raw::Role = CREATE_ROLE(payload: CreateRolePayload) => {
            payload
        },
        /// Fetches a role by its ID.
        get_role(role_id: u64) -> raw::Role = GET_ROLE,
        /// Edits a role by its ID.
        edit_role(role_id: u64) -> raw::Role = EDIT_ROLE(payload: EditRolePayload) => payload,
        /// Deletes a role by its ID.
        delete_role(role_id: u64) -> () = DELETE_ROLE,
        /// Fetches the authenticated user.
        get_authenticated_user() -> raw::ClientUser = GET_AUTHENTICATED_USER,
        /// Edits the authenticated user.
        edit_user() -> raw::ClientUser = EDIT_USER(payload: EditUserPayload) => payload,
        /// Deletes the authenticated user given the correct password.
        delete_user() -> () = DELETE_USER(password: impl AsRef<str>) => {
            DeleteUserPayload {
                password: password.as_ref().to_string(),
            }
        },
        /// Fetches a user by its ID.
        get_user(user_id: u64) -> raw::User = GET_USER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::println;

    #[tokio::test]
    async fn get_channel() -> crate::Result<()> {
        let http = Http::login("crypteballs2@gmail.com", "crypte", Default::default()).await?;
        println!("{}", http.token());

        println!("{:#?}", http.get_authenticated_user().await);
        Ok(())
    }
}
