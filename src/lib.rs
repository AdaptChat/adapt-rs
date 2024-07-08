#![doc = include_str!("../README.md")]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![feature(macro_metavar_expr)]

#[macro_use]
extern crate log;

pub mod client;
mod error;
pub mod http;
pub mod models;
mod server;
#[cfg(feature = "ws")]
pub mod ws;

pub use client::{Context, WithCtx};
pub use error::{Error, Result};
pub use essence;
pub use server::Server;

pub mod prelude {
    pub use super::client::{Context, WithCtx};
    pub use super::essence;
}
