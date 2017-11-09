#[allow(unused)]
use std::ascii::AsciiExt;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use bytes::Bytes;

use byte_str::ByteStr;
use super::{ErrorKind, InvalidUri, InvalidUriBytes, URI_CHARS};

/// Represents the authority component of a URI.
#[derive(Clone)]
pub struct Authority {
    pub(super) data: ByteStr,
}

impl Authority {
    pub(super) fn empty() -> Self {
        Authority { data: ByteStr::new() }
    }

    /// Attempt to convert an `Authority` from `Bytes`.
    ///
    /// This function will be replaced by a `TryFrom` implementation once the
    /// trait lands in stable.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate http;
    /// # use http::uri::*;
    /// extern crate bytes;
    ///
    /// use bytes::Bytes;
    ///
    /// # pub fn main() {
    /// let bytes = Bytes::from("example.com");
    /// let authority = Authority::from_shared(bytes).unwrap();
    ///
    /// assert_eq!(authority.host(), "example.com");
    /// # }
    /// ```
    pub fn from_shared(s: Bytes) -> Result<Self, InvalidUriBytes> {
        let authority_end = Authority::parse(&s[..]).map_err(InvalidUriBytes)?;

        if authority_end != s.len() {
            return Err(ErrorKind::InvalidUriChar.into());
        }

        Ok(Authority {
            data: unsafe { ByteStr::from_utf8_unchecked(s) },
        })
    }

    pub(super) fn parse(s: &[u8]) -> Result<usize, InvalidUri> {
        let mut start_bracket = false;
        let mut end_bracket = false;
        let mut end = s.len();

        for (i, &b) in s.iter().enumerate() {
            match URI_CHARS[b as usize] {
                b'/' | b'?' | b'#' => {
                    end = i;
                    break;
                }
                b'[' => {
                    start_bracket = true;
                }
                b']' => {
                    end_bracket = true;
                }
                0 => {
                    return Err(ErrorKind::InvalidUriChar.into());
                }
                _ => {}
            }
        }

        if start_bracket ^ end_bracket {
            return Err(ErrorKind::InvalidAuthority.into());
        }

        Ok(end)
    }

    /// Get the host of this `Authority`.
    ///
    /// The host subcomponent of authority is identified by an IP literal
    /// encapsulated within square brackets, an IPv4 address in dotted- decimal
    /// form, or a registered name.  The host subcomponent is **case-insensitive**.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                         |---------|
    ///                              |
    ///                             host
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::*;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// assert_eq!(authority.host(), "example.org");
    /// ```
    #[inline]
    pub fn host(&self) -> &str {
        host(self.as_str())
    }

    /// Get the port of this `Authority`.
    ///
    /// The port subcomponent of authority is designated by an optional port
    /// number in decimal following the host and delimited from it by a single
    /// colon (":") character. A value is only returned if one is specified in
    /// the URI string, i.e., default port values are **not** returned.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                                     |-|
    ///                                      |
    ///                                     port
    /// ```
    ///
    /// # Examples
    ///
    /// Authority with port
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// assert_eq!(authority.port(), Some(80));
    /// ```
    ///
    /// Authority without port
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org".parse().unwrap();
    ///
    /// assert!(authority.port().is_none());
    /// ```
    pub fn port(&self) -> Option<u16> {
        let s = self.as_str();
        s.rfind(":").and_then(|i| {
            u16::from_str(&s[i+1..]).ok()
        })
    }

    /// Return a str representation of the authority
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.data[..]
    }

    /// Converts this `Authority` back to a sequence of bytes
    #[inline]
    pub fn into_bytes(self) -> Bytes {
        self.into()
    }
}

impl AsRef<str> for Authority {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for Authority {
    fn eq(&self, other: &Authority) -> bool {
        self.data.eq_ignore_ascii_case(&other.data)
    }
}

impl Eq for Authority {}

/// Case-insensitive equality
///
/// # Examples
///
/// ```
/// # use http::uri::Authority;
/// let authority: Authority = "HELLO.com".parse().unwrap();
/// assert_eq!(authority, *"hello.coM");
/// ```
impl PartialEq<str> for Authority {
    fn eq(&self, other: &str) -> bool {
        self.data.eq_ignore_ascii_case(other)
    }
}

/// Case-insensitive hashing
///
/// # Examples
///
/// ```
/// # use http::uri::Authority;
/// # use std::hash::{Hash, Hasher};
/// # use std::collections::hash_map::DefaultHasher;
///
/// let a: Authority = "HELLO.com".parse().unwrap();
/// let b: Authority = "hello.coM".parse().unwrap();
///
/// let mut s = DefaultHasher::new();
/// a.hash(&mut s);
/// let a = s.finish();
///
/// let mut s = DefaultHasher::new();
/// b.hash(&mut s);
/// let b = s.finish();
///
/// assert_eq!(a, b);
/// ```
impl Hash for Authority {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.data.len().hash(state);
        for &b in self.data.as_bytes() {
            state.write_u8(b.to_ascii_lowercase());
        }
    }
}

impl FromStr for Authority {
    type Err = InvalidUri;

    fn from_str(s: &str) -> Result<Self, InvalidUri> {
        let end = Authority::parse(s.as_bytes())?;

        if end != s.len() {
            return Err(ErrorKind::InvalidAuthority.into());
        }

        Ok(Authority { data: s.into() })
    }
}

impl From<Authority> for Bytes {
    #[inline]
    fn from(src: Authority) -> Bytes {
        src.data.into()
    }
}

impl fmt::Debug for Authority {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Authority {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn host(auth: &str) -> &str {
    let host_port = auth.rsplitn(2, '@')
        .next()
        .expect("split always has at least 1 item");
    if host_port.as_bytes()[0] == b'[' {
        let i = host_port.find(']')
            .expect("parsing should validate brackets");
        &host_port[1..i]
    } else {
        host_port.split(':')
            .next()
            .expect("split always has at least 1 item")
    }
}
