use std::fmt;

use super::{ErrorKind, InvalidUri};

/// The port component of a URI.
pub struct Port {
    port: u16,
    repr: String,
}

impl Port {
    /// Returns the port number as a `u16`.
    ///
    /// # Examples
    ///
    /// Port as `u16`.
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// let port = authority.port().unwrap();
    /// assert_eq!(port.as_u16(), 80);
    /// ```
    pub const fn as_u16(&self) -> u16 {
        self.port
    }
}

impl Port {
    /// Converts a `str` to a port number.
    ///
    /// The supplied `str` must be a valid u16.
    pub(crate) fn from_str(bytes: impl AsRef<str>) -> Result<Self, InvalidUri> {
        bytes
            .as_ref()
            .parse::<u16>()
            .map(|port| Port {
                port,
                repr: bytes.as_ref().to_string(),
            })
            .map_err(|_| ErrorKind::InvalidPort.into())
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
    /// let port = authority.port().unwrap();
    /// assert_eq!(port.as_str(), "80");
    /// ```
    pub fn as_str(&self) -> &str {
        self.repr.as_ref()
    }
}

impl fmt::Debug for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Port").field(&self.port).finish()
    }
}

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use `u16::fmt` so that it respects any formatting flags that
        // may have been set (like padding, align, etc).
        fmt::Display::fmt(&self.port, f)
    }
}

impl From<Port> for u16 {
    fn from(port: Port) -> Self {
        port.as_u16()
    }
}

impl PartialEq<Port> for Port {
    fn eq(&self, other: &Port) -> bool {
        self.port == other.port
    }
}

impl PartialEq<u16> for Port {
    fn eq(&self, other: &u16) -> bool {
        self.port == *other
    }
}

impl PartialEq<Port> for u16 {
    fn eq(&self, other: &Port) -> bool {
        other.port == *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partialeq_port() {
        let port_a = Port::from_str("8080").unwrap();
        let port_b = Port::from_str("8080").unwrap();
        assert_eq!(port_a, port_b);
    }

    #[test]
    fn partialeq_port_different_reprs() {
        let port_a = Port {
            repr: "8081".to_string(),
            port: 8081,
        };
        let port_b = Port {
            repr: String::from("8081"),
            port: 8081,
        };
        assert_eq!(port_a, port_b);
        assert_eq!(port_b, port_a);
    }

    #[test]
    fn partialeq_u16() {
        let port = Port::from_str("8080").unwrap();
        // test equals in both directions
        assert_eq!(port, 8080);
        assert_eq!(8080, port);
    }

    #[test]
    fn u16_from_port() {
        let port = Port::from_str("8080").unwrap();
        assert_eq!(8080, u16::from(port));
    }
}
