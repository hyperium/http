//! HTTP version
use std::fmt;

/// Represents a version of the HTTP spec.
#[derive(PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash, Debug)]
pub struct Version(Http);

/// `HTTP/0.9`
pub const HTTP_09: Version = Version(Http::Http09);

/// `HTTP/1.0`
pub const HTTP_10: Version = Version(Http::Http10);

/// `HTTP/1.1`
pub const HTTP_11: Version = Version(Http::Http11);

/// `HTTP/2.0` over TLS
pub const HTTP_2: Version = Version(Http::H2);

/// `HTTP/2.0` over cleartext
pub const HTTP_2C: Version = Version(Http::H2c);

#[derive(PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash, Debug)]
enum Http {
    Http09,
    Http10,
    Http11,
    H2,
    H2c,
}

/// A possible error value when converting `Version` from bytes.
#[derive(Debug)]
pub struct FromBytesError {
    _priv: (),
}

impl Version {
    /// Converts a slice of bytes to an HTTP version.
    pub fn from_bytes(bytes: &[u8]) -> Result<Version, FromBytesError> {
        match bytes {
            b"HTTP/0.9" => Ok(HTTP_09),
            b"HTTP/1.0" => Ok(HTTP_10),
            b"HTTP/1.1" => Ok(HTTP_11),
            b"h2"       => Ok(HTTP_2),
            b"h2c"      => Ok(HTTP_2C),
            _           => Err(FromBytesError { _priv: (), }),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(match self.0 {
            Http::Http09 => "HTTP/0.9",
            Http::Http10 => "HTTP/1.0",
            Http::Http11 => "HTTP/1.1",
            Http::H2 => "h2",
            Http::H2c => "h2c",
        })
    }
}

impl Default for Version {
    fn default() -> Version {
        HTTP_11
    }
}
