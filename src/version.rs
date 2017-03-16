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

impl Version {
    /// Returns a number indicating the major version number.
    ///
    /// # Example
    ///
    /// ```rust
    /// assert_eq!(http::Version::http_11().major(), 1);
    /// ```
    pub fn major(&self) -> u8 {
        match self.0 {
            Http::Http09 => 0,
            Http::Http10 |
            Http::Http11 => 1,
            Http::H2     |
            Http::H2c    => 2,
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
