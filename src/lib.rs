//! Common HTTP types.

#![deny(warnings, missing_docs, missing_debug_implementations)]

extern crate bytes;
extern crate fnv;

pub mod header;
pub mod method;
pub mod request;
pub mod response;
pub mod status;
pub mod version;
pub mod uri;

mod byte_str;
mod convert;
mod error;

pub use error::{Error, Result};
pub use convert::HttpTryFrom;
pub use header::HeaderMap;
pub use method::Method;
pub use request::Request;
pub use response::Response;
pub use status::StatusCode;
pub use uri::Uri;
pub use version::Version;

mod sealed {
    /// Private trait to this crate to prevent traits from being implemented in
    /// downstream crates.
    pub trait Sealed {}
}
