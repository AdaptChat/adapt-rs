//! Low-level API wrapper for the Adapt REST and WebSocket APIs.

extern crate core;

mod error;
pub mod http;
#[cfg(feature = "ws")]
pub mod ws;

pub use error::{Error, Result};
pub use essence;
