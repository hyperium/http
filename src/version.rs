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

use std::fmt;

/// Represents a version of the HTTP spec.
#[derive(PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash)]
pub struct Version(Http);

impl Version {
    /// `HTTP/0.9`
    pub const HTTP_09: Version = Version(Http::Http09);

    /// `HTTP/1.0`
    pub const HTTP_10: Version = Version(Http::Http10);

    /// `HTTP/1.1`
    pub const HTTP_11: Version = Version(Http::Http11);

    /// `HTTP/2.0`
    pub const HTTP_2: Version = Version(Http::H2);

    /// `HTTP/3.0`
    pub const HTTP_3: Version = Version(Http::H3);

    pub(crate) fn as_str(&self) -> &'static str {
        use self::Http::*;
        match self.0 {
            Http09 => "HTTP/0.9",
            Http10 => "HTTP/1.0",
            Http11 => "HTTP/1.1",
            H2 => "HTTP/2.0",
            H3 => "HTTP/3.0",
            __NonExhaustive => unreachable!(),
        }
    }
}

#[derive(PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash)]
enum Http {
    Http09,
    Http10,
    Http11,
    H2,
    H3,
    __NonExhaustive,
}

impl Default for Version {
    #[inline]
    fn default() -> Version {
        Version::HTTP_11
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(feature = "serde1")]
mod serde1 {
    use std::fmt;

    use serde::{de, Deserialize};

    use super::{Http, Version};

    struct VersionVisitor;

    impl<'de> de::Visitor<'de> for VersionVisitor {
        type Value = Version;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a version string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let http = match v {
                "HTTP/0.9" => Http::Http09,
                "HTTP/1.0" => Http::Http10,
                "HTTP/1.1" => Http::Http11,
                "HTTP/2.0" => Http::H2,
                "HTTP/3.0" => Http::H3,
                _ => return Err(E::invalid_value(de::Unexpected::Str(v), &self)),
            };
            Ok(Version(http))
        }
    }

    impl<'de> Deserialize<'de> for Version {
        fn deserialize<D>(deserializer: D) -> Result<Version, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            deserializer.deserialize_str(VersionVisitor)
        }
    }
}
