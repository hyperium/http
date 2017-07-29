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
mod extensions;

pub use convert::HttpTryFrom;
pub use error::{Error, Result};
pub use extensions::Extensions;
pub use header::HeaderMap;
pub use method::Method;
pub use request::{Request, RequestBuilder};
pub use response::{Response, ResponseBuilder};
pub use status::StatusCode;
pub use uri::Uri;
pub use version::Version;

fn _assert_types() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<Request<()>>();
    assert_send::<Response<()>>();

    assert_sync::<Request<()>>();
    assert_sync::<Response<()>>();
}

mod sealed {
    /// Private trait to this crate to prevent traits from being implemented in
    /// downstream crates.
    pub trait Sealed {}
}
