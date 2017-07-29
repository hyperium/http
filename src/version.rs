//! HTTP version
//!
//! This module contains a definition of the `Version` type and various
//! constants associated with it as well. The `Version` type is intended to be
//! accessed through the root of the crate (`http::Version`) rather than this
//! module.
//!
//! # Examples
//!
//! ```
//! use http::{version, Version};
//!
//! let http11 = version::HTTP_11;
//! let http2 = version::HTTP_2;
//! assert!(http11 != http2);
//!
//! println!("{:?}", http2);
//! ```

use std::fmt;

/// Represents a version of the HTTP spec.
#[derive(PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash)]
pub struct Version(Http);

/// `HTTP/0.9`
pub const HTTP_09: Version = Version(Http::Http09);

/// `HTTP/1.0`
pub const HTTP_10: Version = Version(Http::Http10);

/// `HTTP/1.1`
pub const HTTP_11: Version = Version(Http::Http11);

/// `HTTP/2.0`
pub const HTTP_2: Version = Version(Http::H2);

#[derive(PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash)]
enum Http {
    Http09,
    Http10,
    Http11,
    H2,
}

impl Default for Version {
    #[inline]
    fn default() -> Version {
        HTTP_11
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Http::*;

        f.write_str(match self.0 {
            Http09 => "HTTP/0.9",
            Http10 => "HTTP/1.0",
            Http11 => "HTTP/1.1",
            H2     => "HTTP/2.0",
        })
    }
}
