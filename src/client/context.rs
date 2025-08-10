use crate::http::Http;
#[cfg(feature = "ws")]
use crate::ws::Messenger;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// Allows access to shared values regarding the client state, including the HTTP client, gateway
/// connection, and cache.
///
/// Context is typically attached to a value using the [`WithCtx`] type.
#[derive(Clone)]
pub struct Context {
    /// The HTTP client used to make requests to the REST API.
    pub(crate) http: Arc<Http>,
    /// The messenger for the connection to Harmony.
    #[cfg(feature = "ws")]
    pub(crate) ws: Option<Messenger>,
}

impl Context {
    /// Creates an ad-hoc [`Context`] from an HTTP client.
    pub fn from_http(http: Arc<Http>) -> Self {
        Self {
            http,
            ws: None,
        }
    }
    
    /// Returns a reference to the HTTP client, used to make requests to the REST API.
    #[must_use]
    pub const fn http(&self) -> &Arc<Http> {
        &self.http
    }

    /// Returns a reference to the websocket messenger. This is `None` if there is no active
    /// connection to Harmony yet.
    #[cfg(feature = "ws")]
    #[must_use]
    pub const fn ws(&self) -> Option<&Messenger> {
        self.ws.as_ref()
    }

    /// Wraps a value with the current context using [`WithCtx`].
    pub const fn with<T>(self, inner: T) -> WithCtx<T> {
        WithCtx { inner, ctx: self }
    }
}

impl Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").finish()
    }
}

/// A type that wraps a value with a [`Context`]. This extends functionality of Adapt objects by
/// allowing them to access shared client state.
///
/// # See Also
/// - [`Context`]: The client state itself.
#[derive(Clone)]
#[must_use]
pub struct WithCtx<T> {
    /// The value being wrapped.
    pub(crate) inner: T,
    /// The context to attached to the value.
    pub ctx: Context,
}

impl<T> WithCtx<T> {
    /// Consumes the [`WithCtx`] instance, returning the inner value.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Borrows the inner value.
    pub const fn inner(&self) -> &T {
        &self.inner
    }

    /// Borrows the inner value mutably.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T> Deref for WithCtx<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for WithCtx<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: Debug> Debug for WithCtx<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WithCtx").field(&self.inner).finish()
    }
}
