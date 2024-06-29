#![doc = include_str!("../README.md")]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![feature(macro_metavar_expr)]

mod error;
pub mod http;
#[cfg(feature = "ws")]
pub mod ws;

pub use error::{Error, Result};
pub use essence;
