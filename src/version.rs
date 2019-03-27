//! HTTP version
//!
//! This module contains a definition of the `Version` type. The `Version`
//! type is intended to be accessed through the root of the crate
//! (`http::Version`) rather than this module.
//!
//! The `Version` type contains constants that represent the various versions
//! of the HTTP protocol.
//!
//! # Examples
//!
//! ```
//! use http::Version;
//!
//! let http11 = Version::HTTP_11;
//! let http2 = Version::HTTP_2;
//! assert!(http11 != http2);
//!
//! println!("{:?}", http2);
//! ```

use std::fmt::{self, Display, Formatter};

/// Represents a version of the HTTP spec.
#[derive(Debug, PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash)]
pub struct Version(Http);

impl Version {
    /// `HTTP/0.9`
    pub const HTTP_09: Version = Version(Http::Http09);

    /// `HTTP/1.0`
    pub const HTTP_10: Version = Version(Http::Http10);

    /// `HTTP/1.1`
    pub const HTTP_11: Version = Version(Http::Http11);

    /// `HTTP/2.0`
    pub const HTTP_2: Version = Version(Http::Http2);
}

#[derive(Debug, PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash)]
enum Http {
    Http09,
    Http10,
    Http11,
    Http2,
}

impl Display for Http {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match self {
            Http::Http09 => write!(fmt, "HTTP/0.9"),
            Http::Http10 => write!(fmt, "HTTP/1.0"),
            Http::Http11 => write!(fmt, "HTTP/1.1"),
            Http::Http2 => write!(fmt, "HTTP/2.0"),
        }
    }
}

impl Default for Version {
    #[inline]
    fn default() -> Version {
        Version::HTTP_11
    }
}

impl Display for Version {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "{}", self.0)
    }
}