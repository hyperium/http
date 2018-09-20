use std::{fmt, num};
use std::str::FromStr;

/// The port component of a URI.
#[derive(Debug)]
pub struct Port<'a> {
    bytes: &'a str,
    port: u16,
}

impl<'a> Port<'a> {
    /// Converts a `str` to a port number.
    ///
    /// The supplied `str` must be a valid u16.
    pub(crate) fn from_str(bytes: &'a str) -> Result<Self, num::ParseIntError> {
        u16::from_str(bytes).and_then(|port| Ok(Port { port, bytes }))
    }

    /// Returns the port number as a `u16`.
    ///
    /// # Examples
    ///
    /// Port as `u16`
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// let port = authority.port_part().unwrap();
    /// assert_eq!(port.as_u16(), 80);
    /// ```
    pub fn as_u16(&self) -> u16 {
        self.port
    }

    /// Returns the port number as a `str`.
    ///
    /// # Examples
    ///
    /// Port as `str`.
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// let port = authority.port_part().unwrap();
    /// assert_eq!(port.as_str(), "80");
    /// ```
    pub fn as_str(&self) -> &'a str {
        self.bytes
    }
}

impl<'a> PartialEq for Port<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.port == other.port
    }
}

impl<'a> fmt::Display for Port<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.bytes)
    }
}
