//! HTTP version

/// Represents a version of the HTTP spec.
#[derive(PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash, Debug)]
pub struct Version(Http);

/// `HTTP/0.9`
pub const HTTP_09: Version = Version(Http::Http09);

/// `HTTP/1.0`
pub const HTTP_10: Version = Version(Http::Http10);

/// `HTTP/1.1`
pub const HTTP_11: Version = Version(Http::Http11);

/// `HTTP/2.0`
pub const HTTP_2: Version = Version(Http::H2);

#[derive(PartialEq, PartialOrd, Copy, Clone, Eq, Ord, Hash, Debug)]
enum Http {
    Http09,
    Http10,
    Http11,
    H2,
}

impl Default for Version {
    fn default() -> Version {
        HTTP_11
    }
}
