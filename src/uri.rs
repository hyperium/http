//! URI component of request and response lines
//!
//! This module primarily contains the `Uri` type which is a component of all
//! HTTP requests and also reexports this type at the root of the crate. A URI
//! is not always a "full URL" in the sense of something you'd type into a web
//! browser, but HTTP requests may only have paths on servers but may have full
//! schemes and hostnames on clients.
//!
//! # Examples
//!
//! ```
//! use http::Uri;
//!
//! let uri = "/foo/bar?baz".parse::<Uri>().unwrap();
//! assert_eq!(uri.path(), "/foo/bar");
//! assert_eq!(uri.query(), Some("baz"));
//! assert_eq!(uri.host(), None);
//!
//! let uri = "https://www.rust-lang.org/install.html".parse::<Uri>().unwrap();
//! assert_eq!(uri.scheme(), Some("https"));
//! assert_eq!(uri.host(), Some("www.rust-lang.org"));
//! assert_eq!(uri.path(), "/install.html");
//! ```

use HttpTryFrom;
use byte_str::ByteStr;

use bytes::Bytes;

use std::{fmt, u8, u16};
#[allow(unused)]
use std::ascii::AsciiExt;
use std::hash::{Hash, Hasher};
use std::str::{self, FromStr};
use std::error::Error;

/// The URI component of a request.
///
/// For HTTP 1, this is included as part of the request line. From Section 5.3,
/// Request Target:
///
/// > Once an inbound connection is obtained, the client sends an HTTP
/// > request message (Section 3) with a request-target derived from the
/// > target URI.  There are four distinct formats for the request-target,
/// > depending on both the method being requested and whether the request
/// > is to a proxy.
/// >
/// > ```notrust
/// > request-target = origin-form
/// >                / absolute-form
/// >                / authority-form
/// >                / asterisk-form
/// > ```
///
/// The URI is structured as follows:
///
/// ```notrust
/// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
/// |-|   |-------------------------------||--------| |-------------------| |-----|
///  |                  |                       |               |              |
/// scheme          authority                 path            query         fragment
/// ```
///
/// For HTTP 2.0, the URI is encoded using pseudoheaders.
///
/// # Examples
///
/// ```
/// use http::Uri;
///
/// let uri = "/foo/bar?baz".parse::<Uri>().unwrap();
/// assert_eq!(uri.path(), "/foo/bar");
/// assert_eq!(uri.query(), Some("baz"));
/// assert_eq!(uri.host(), None);
///
/// let uri = "https://www.rust-lang.org/install.html".parse::<Uri>().unwrap();
/// assert_eq!(uri.scheme(), Some("https"));
/// assert_eq!(uri.host(), Some("www.rust-lang.org"));
/// assert_eq!(uri.path(), "/install.html");
/// ```
#[derive(Clone)]
pub struct Uri {
    scheme: Scheme,
    authority: Authority,
    path_and_query: PathAndQuery,
}

/// Represents the scheme component of a URI
#[derive(Clone)]
pub struct Scheme {
    inner: Scheme2,
}

#[derive(Clone, Debug)]
enum Scheme2<T = Box<ByteStr>> {
    None,
    Standard(Protocol),
    Other(T),
}

#[derive(Copy, Clone, Debug)]
enum Protocol {
    Http,
    Https,
}

/// Represents the authority component of a URI.
#[derive(Clone)]
pub struct Authority {
    data: ByteStr,
}

/// Represents the path component of a URI
#[derive(Clone)]
pub struct PathAndQuery {
    data: ByteStr,
    query: u16,
}

/// The various parts of a URI.
///
/// This struct is used to provide to and retrieve from a URI.
#[derive(Debug, Default)]
pub struct Parts {
    /// The scheme component of a URI
    pub scheme: Option<Scheme>,

    /// The authority component of a URI
    pub authority: Option<Authority>,

    /// The origin-form component of a URI
    pub path_and_query: Option<PathAndQuery>,

    /// Allow extending in the future
    _priv: (),
}

/// An error resulting from a failed attempt to construct a URI.
#[derive(Debug)]
pub struct InvalidUri(ErrorKind);

/// An error resulting from a failed attempt to construct a URI.
#[derive(Debug)]
pub struct InvalidUriBytes(InvalidUri);

/// An error resulting from a failed attempt to construct a URI.
#[derive(Debug)]
pub struct InvalidUriParts(InvalidUri);

#[derive(Debug, Eq, PartialEq)]
enum ErrorKind {
    InvalidUriChar,
    InvalidScheme,
    InvalidAuthority,
    InvalidFormat,
    AuthorityMissing,
    PathAndQueryMissing,
    TooLong,
    Empty,
    SchemeTooLong,
}

// u16::MAX is reserved for None
const MAX_LEN: usize = (u16::MAX - 1) as usize;

// Require the scheme to not be too long in order to enable further
// optimizations later.
const MAX_SCHEME_LEN: usize = 64;
const NONE: u16 = u16::MAX;

const URI_CHARS: [u8; 256] = [
    //  0      1      2      3      4      5      6      7      8      9
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //   x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  1x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  2x
        0,     0,     0,  b'!',     0,  b'#',  b'$',     0,  b'&', b'\'', //  3x
     b'(',  b')',  b'*',  b'+',  b',',  b'-',  b'.',  b'/',  b'0',  b'1', //  4x
     b'2',  b'3',  b'4',  b'5',  b'6',  b'7',  b'8',  b'9',  b':',  b';', //  5x
        0,  b'=',     0,  b'?',  b'@',  b'A',  b'B',  b'C',  b'D',  b'E', //  6x
     b'F',  b'G',  b'H',  b'I',  b'J',  b'K',  b'L',  b'M',  b'N',  b'O', //  7x
     b'P',  b'Q',  b'R',  b'S',  b'T',  b'U',  b'V',  b'W',  b'X',  b'Y', //  8x
     b'Z',  b'[',     0,  b']',     0,  b'_',     0,  b'a',  b'b',  b'c', //  9x
     b'd',  b'e',  b'f',  b'g',  b'h',  b'i',  b'j',  b'k',  b'l',  b'm', // 10x
     b'n',  b'o',  b'p',  b'q',  b'r',  b's',  b't',  b'u',  b'v',  b'w', // 11x
     b'x',  b'y',  b'z',     0,     0,     0,  b'~',     0,     0,     0, // 12x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 13x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 14x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 15x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 16x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 17x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 18x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 19x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 20x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 21x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 22x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 23x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 24x
        0,     0,     0,     0,     0,     0                              // 25x
];

// scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
//
const SCHEME_CHARS: [u8; 256] = [
    //  0      1      2      3      4      5      6      7      8      9
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //   x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  1x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  2x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  3x
        0,     0,     0,  b'+',     0,  b'-',  b'.',     0,  b'0',  b'1', //  4x
     b'2',  b'3',  b'4',  b'5',  b'6',  b'7',  b'8',  b'9',  b':',     0, //  5x
        0,     0,     0,     0,     0,  b'A',  b'B',  b'C',  b'D',  b'E', //  6x
     b'F',  b'G',  b'H',  b'I',  b'J',  b'K',  b'L',  b'M',  b'N',  b'O', //  7x
     b'P',  b'Q',  b'R',  b'S',  b'T',  b'U',  b'V',  b'W',  b'X',  b'Y', //  8x
     b'Z',     0,     0,     0,     0,     0,     0,  b'a',  b'b',  b'c', //  9x
     b'd',  b'e',  b'f',  b'g',  b'h',  b'i',  b'j',  b'k',  b'l',  b'm', // 10x
     b'n',  b'o',  b'p',  b'q',  b'r',  b's',  b't',  b'u',  b'v',  b'w', // 11x
     b'x',  b'y',  b'z',     0,     0,     0,  b'~',     0,     0,     0, // 12x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 13x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 14x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 15x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 16x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 17x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 18x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 19x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 20x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 21x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 22x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 23x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 24x
        0,     0,     0,     0,     0,     0                              // 25x
];

const HEX_DIGIT: [u8; 256] = [
    //  0      1      2      3      4      5      6      7      8      9
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //   x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  1x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  2x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  3x
        0,     0,     0,     0,     0,     0,     0,     0,  b'0',  b'1', //  4x
     b'2',  b'3',  b'4',  b'5',  b'6',  b'7',  b'8',  b'9',     0,     0, //  5x
        0,     0,     0,     0,     0,  b'A',  b'B',  b'C',  b'D',  b'E', //  6x
     b'F',  b'G',  b'H',  b'I',  b'J',  b'K',  b'L',  b'M',  b'N',  b'O', //  7x
     b'P',  b'Q',  b'R',  b'S',  b'T',  b'U',  b'V',  b'W',  b'X',  b'Y', //  8x
     b'Z',     0,     0,     0,     0,     0,     0,  b'a',  b'b',  b'c', //  9x
     b'd',  b'e',  b'f',  b'g',  b'h',  b'i',  b'j',  b'k',  b'l',  b'm', // 10x
     b'n',  b'o',  b'p',  b'q',  b'r',  b's',  b't',  b'u',  b'v',  b'w', // 11x
     b'x',  b'y',  b'z',     0,     0,     0,  b'~',     0,     0,     0, // 12x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 13x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 14x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 15x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 16x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 17x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 18x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 19x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 20x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 21x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 22x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 23x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 24x
        0,     0,     0,     0,     0,     0                              // 25x
];

impl Uri {
    /// Attempt to convert a `Uri` from `Parts`
    pub fn from_parts(src: Parts) -> Result<Uri, InvalidUriParts> {
        if src.scheme.is_some() {
            if src.authority.is_none() {
                return Err(ErrorKind::AuthorityMissing.into());
            }

            if src.path_and_query.is_none() {
                return Err(ErrorKind::PathAndQueryMissing.into());
            }
        } else {
            if src.authority.is_some() && src.path_and_query.is_none() {
                return Err(ErrorKind::PathAndQueryMissing.into());
            }
        }

        let scheme = match src.scheme {
            Some(scheme) => scheme,
            None => Scheme { inner: Scheme2::None },
        };

        let authority = match src.authority {
            Some(authority) => authority,
            None => Authority::empty(),
        };

        let path_and_query = match src.path_and_query {
            Some(path_and_query) => path_and_query,
            None => PathAndQuery::empty(),
        };

        Ok(Uri {
            scheme: scheme,
            authority: authority,
            path_and_query: path_and_query,
        })
    }

    /// Attempt to convert a `Uri` from `Bytes`
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
    /// let bytes = Bytes::from("http://example.com/foo");
    /// let uri = Uri::from_shared(bytes).unwrap();
    ///
    /// assert_eq!(uri.host().unwrap(), "example.com");
    /// assert_eq!(uri.path(), "/foo");
    /// # }
    /// ```
    pub fn from_shared(s: Bytes) -> Result<Uri, InvalidUriBytes> {
        use self::ErrorKind::*;

        if s.len() > MAX_LEN {
            return Err(TooLong.into());
        }

        match s.len() {
            0 => {
                return Err(Empty.into());
            }
            1 => {
                match s[0] {
                    b'/' => {
                        return Ok(Uri {
                            scheme: Scheme::empty(),
                            authority: Authority::empty(),
                            path_and_query: PathAndQuery::slash(),
                        });
                    }
                    b'*' => {
                        return Ok(Uri {
                            scheme: Scheme::empty(),
                            authority: Authority::empty(),
                            path_and_query: PathAndQuery::star(),
                        });
                    }
                    _ => {
                        let authority = Authority::from_shared(s)?;

                        return Ok(Uri {
                            scheme: Scheme::empty(),
                            authority: authority,
                            // TODO: Should this be empty instead?
                            path_and_query: PathAndQuery::slash(),
                        });
                    }
                }
            }
            _ => {}
        }

        if s[0] == b'/' {
            return Ok(Uri {
                scheme: Scheme::empty(),
                authority: Authority::empty(),
                path_and_query: PathAndQuery::from_shared(s)?,
            });
        }

        parse_full(s)
    }

    /// Returns the path & query components of the Uri
    #[inline]
    pub fn path_and_query(&self) -> Option<&PathAndQuery> {
        if !self.scheme.inner.is_none() || self.authority.data.is_empty() {
            Some(&self.path_and_query)
        } else {
            None
        }
    }

    /// Get the path of this `Uri`.
    ///
    /// Both relative and absolute URIs contain a path component, though it
    /// might be the empty string. The path component is **case sensitive**.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                                        |--------|
    ///                                             |
    ///                                           path
    /// ```
    ///
    /// If the URI is `*` then the path component is equal to `*`.
    ///
    /// # Examples
    ///
    /// A relative URI
    ///
    /// ```
    /// # use http::Uri;
    ///
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.path(), "/hello/world");
    /// ```
    ///
    /// An absolute URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.path(), "/hello/world");
    /// ```
    #[inline]
    pub fn path(&self) -> &str {
        if self.has_path() {
            self.path_and_query.path()
        } else {
            ""
        }
    }

    /// Get the scheme of this `Uri`.
    ///
    /// The URI scheme refers to a specification for assigning identifiers
    /// within that scheme. Only absolute URIs contain a scheme component, but
    /// not all absolute URIs will contain a scheme component.  Although scheme
    /// names are case-insensitive, the canonical form is lowercase.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    /// |-|
    ///  |
    /// scheme
    /// ```
    ///
    /// # Examples
    ///
    /// Absolute URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.scheme(), Some("http"));
    /// ```
    ///
    ///
    /// Relative URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert!(uri.scheme().is_none());
    /// ```
    #[inline]
    pub fn scheme(&self) -> Option<&str> {
        if self.scheme.inner.is_none() {
            None
        } else {
            Some(self.scheme.as_str())
        }
    }

    /// Get the authority of this `Uri`.
    ///
    /// The authority is a hierarchical element for naming authority such that
    /// the remainder of the URI is delegated to that authority. For HTTP, the
    /// authority consists of the host and port. The host portion of the
    /// authority is **case-insensitive**.
    ///
    /// The authority also includes a `username:password` component, however
    /// the use of this is deprecated and should be avoided.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///       |-------------------------------|
    ///                     |
    ///                 authority
    /// ```
    ///
    /// This function will be renamed to `authority` in the next semver release.
    ///
    /// # Examples
    ///
    /// Absolute URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org:80/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.authority_part().map(|a| a.as_str()), Some("example.org:80"));
    /// ```
    ///
    ///
    /// Relative URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert!(uri.authority_part().is_none());
    /// ```
    #[inline]
    pub fn authority_part(&self) -> Option<&Authority> {
        if self.authority.data.is_empty() {
            None
        } else {
            Some(&self.authority)
        }
    }

    #[deprecated(since = "0.1.1", note = "use authority_part instead")]
    #[doc(hidden)]
    #[inline]
    pub fn authority(&self) -> Option<&str> {
        if self.authority.data.is_empty() {
            None
        } else {
            Some(self.authority.as_str())
        }
    }

    /// Get the host of this `Uri`.
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
    /// Absolute URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org:80/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.host(), Some("example.org"));
    /// ```
    ///
    ///
    /// Relative URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert!(uri.host().is_none());
    /// ```
    #[inline]
    pub fn host(&self) -> Option<&str> {
        self.authority_part().map(|a| a.host())
    }

    /// Get the port of this `Uri`.
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
    /// Absolute URI with port
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org:80/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.port(), Some(80));
    /// ```
    ///
    /// Absolute URI without port
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org/hello/world".parse().unwrap();
    ///
    /// assert!(uri.port().is_none());
    /// ```
    ///
    /// Relative URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert!(uri.port().is_none());
    /// ```
    pub fn port(&self) -> Option<u16> {
        self.authority_part()
            .and_then(|a| a.port())
    }

    /// Get the query string of this `Uri`, starting after the `?`.
    ///
    /// The query component contains non-hierarchical data that, along with data
    /// in the path component, serves to identify a resource within the scope of
    /// the URI's scheme and naming authority (if any). The query component is
    /// indicated by the first question mark ("?") character and terminated by a
    /// number sign ("#") character or by the end of the URI.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                                                   |-------------------|
    ///                                                             |
    ///                                                           query
    /// ```
    ///
    /// # Examples
    ///
    /// Absolute URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org/hello/world?key=value".parse().unwrap();
    ///
    /// assert_eq!(uri.query(), Some("key=value"));
    /// ```
    ///
    /// Relative URI with a query string component
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world?key=value&foo=bar".parse().unwrap();
    ///
    /// assert_eq!(uri.query(), Some("key=value&foo=bar"));
    /// ```
    ///
    /// Relative URI without a query string component
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert!(uri.query().is_none());
    /// ```
    #[inline]
    pub fn query(&self) -> Option<&str> {
        self.path_and_query.query()
    }

    fn has_path(&self) -> bool {
        !self.path_and_query.data.is_empty() || !self.scheme.inner.is_none()
    }
}

impl<'a> HttpTryFrom<&'a str> for Uri {
    type Error = InvalidUri;

    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        t.parse()
    }
}

impl HttpTryFrom<Bytes> for Uri {
    type Error = InvalidUriBytes;

    #[inline]
    fn try_from(t: Bytes) -> Result<Self, Self::Error> {
        Uri::from_shared(t)
    }
}

impl HttpTryFrom<Parts> for Uri {
    type Error = InvalidUriParts;

    #[inline]
    fn try_from(src: Parts) -> Result<Self, Self::Error> {
        Uri::from_parts(src)
    }
}

/// Convert a `Uri` from parts
///
/// # Examples
///
/// Relative URI
///
/// ```
/// # use http::uri::*;
/// let mut parts = Parts::default();
/// parts.path_and_query = Some("/foo".parse().unwrap());
///
/// let uri = Uri::from_parts(parts).unwrap();
///
/// assert_eq!(uri.path(), "/foo");
///
/// assert!(uri.scheme().is_none());
/// assert!(uri.authority().is_none());
/// ```
///
/// Absolute URI
///
/// ```
/// # use http::uri::*;
/// let mut parts = Parts::default();
/// parts.scheme = Some("http".parse().unwrap());
/// parts.authority = Some("foo.com".parse().unwrap());
/// parts.path_and_query = Some("/foo".parse().unwrap());
///
/// let uri = Uri::from_parts(parts).unwrap();
///
/// assert_eq!(uri.scheme().unwrap(), "http");
/// assert_eq!(uri.authority().unwrap(), "foo.com");
/// assert_eq!(uri.path(), "/foo");
/// ```
impl From<Uri> for Parts {
    fn from(src: Uri) -> Self {
        let path_and_query = if src.has_path() {
            Some(src.path_and_query)
        } else {
            None
        };

        let scheme = match src.scheme.inner {
            Scheme2::None => None,
            _ => Some(src.scheme),
        };

        let authority = if src.authority.data.is_empty() {
            None
        } else {
            Some(src.authority)
        };

        Parts {
            scheme: scheme,
            authority: authority,
            path_and_query: path_and_query,
            _priv: (),
        }
    }
}

impl Scheme {
    /// Attempt to convert a `Scheme` from `Bytes`
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
    /// let bytes = Bytes::from("http");
    /// let scheme = Scheme::from_shared(bytes).unwrap();
    ///
    /// assert_eq!(scheme.as_str(), "http");
    /// # }
    /// ```
    pub fn from_shared(s: Bytes) -> Result<Self, InvalidUriBytes> {
        use self::Scheme2::*;

        match Scheme2::parse_exact(&s[..]).map_err(InvalidUriBytes)? {
            None => Err(ErrorKind::InvalidScheme.into()),
            Standard(p) => Ok(Standard(p).into()),
            Other(_) => {
                let b = unsafe { ByteStr::from_utf8_unchecked(s) };
                Ok(Other(Box::new(b)).into())
            }
        }
    }

    fn empty() -> Self {
        Scheme {
            inner: Scheme2::None,
        }
    }

    /// Return a str representation of the scheme
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::*;
    /// let scheme: Scheme = "http".parse().unwrap();
    /// assert_eq!(scheme.as_str(), "http");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        use self::Scheme2::*;
        use self::Protocol::*;

        match self.inner {
            Standard(Http) => "http",
            Standard(Https) => "https",
            Other(ref v) => &v[..],
            None => unreachable!(),
        }
    }

    /// Converts this `Scheme` back to a sequence of bytes
    #[inline]
    pub fn into_bytes(self) -> Bytes {
        self.into()
    }
}

impl FromStr for Scheme {
    type Err = InvalidUri;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use self::Scheme2::*;

        match Scheme2::parse_exact(s.as_bytes())? {
            None => Err(ErrorKind::InvalidScheme.into()),
            Standard(p) => Ok(Standard(p).into()),
            Other(_) => {
                Ok(Other(Box::new(s.into())).into())
            }
        }
    }
}

impl From<Scheme> for Bytes {
    #[inline]
    fn from(src: Scheme) -> Self {
        use self::Scheme2::*;
        use self::Protocol::*;

        match src.inner {
            None => Bytes::new(),
            Standard(Http) => Bytes::from_static(b"http"),
            Standard(Https) => Bytes::from_static(b"https"),
            Other(v) => (*v).into(),
        }
    }
}

impl fmt::Debug for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<T> Scheme2<T> {
    fn is_none(&self) -> bool {
        match *self {
            Scheme2::None => true,
            _ => false,
        }
    }
}

impl Scheme2<usize> {
    fn parse_exact(s: &[u8]) -> Result<Scheme2<()>, InvalidUri> {
        match s {
            b"http" => Ok(Protocol::Http.into()),
            b"https" => Ok(Protocol::Https.into()),
            _ => {
                if s.len() > MAX_SCHEME_LEN {
                    return Err(ErrorKind::SchemeTooLong.into());
                }

                for &b in s {
                    match SCHEME_CHARS[b as usize] {
                        b':' => {
                            // Don't want :// here
                            return Err(ErrorKind::InvalidScheme.into());
                        }
                        0 => {
                            return Err(ErrorKind::InvalidScheme.into());
                        }
                        _ => {}
                    }
                }

                Ok(Scheme2::Other(()))
            }
        }
    }

    fn parse(s: &[u8]) -> Result<Scheme2<usize>, InvalidUri> {
        if s.len() >= 7 {
            // Check for HTTP
            if s[..7].eq_ignore_ascii_case(b"http://") {
                // Prefix will be striped
                return Ok(Protocol::Http.into());
            }
        }

        if s.len() >= 8 {
            // Check for HTTPs
            if s[..8].eq_ignore_ascii_case(b"https://") {
                return Ok(Protocol::Https.into());
            }
        }

        if s.len() > 3 {
            for i in 0..s.len() {
                let b = s[i];

                if i == MAX_SCHEME_LEN {
                    return Err(ErrorKind::SchemeTooLong.into());
                }

                match SCHEME_CHARS[b as usize] {
                    b':' => {
                        // Not enough data remaining
                        if s.len() < i + 3 {
                            break;
                        }

                        // Not a scheme
                        if &s[i+1..i+3] != b"//" {
                            break;
                        }

                        // Return scheme
                        return Ok(Scheme2::Other(i));
                    }
                    // Invald scheme character, abort
                    0 => break,
                    _ => {}
                }
            }
        }

        Ok(Scheme2::None)
    }
}

impl Protocol {
    fn len(&self) -> usize {
        match *self {
            Protocol::Http => 4,
            Protocol::Https => 5,
        }
    }
}

impl Authority {
    fn empty() -> Self {
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

    fn parse(s: &[u8]) -> Result<usize, InvalidUri> {
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

impl PathAndQuery {
    /// Attempt to convert a `PathAndQuery` from `Bytes`.
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
    /// let bytes = Bytes::from("/hello?world");
    /// let path_and_query = PathAndQuery::from_shared(bytes).unwrap();
    ///
    /// assert_eq!(path_and_query.path(), "/hello");
    /// assert_eq!(path_and_query.query(), Some("world"));
    /// # }
    /// ```
    pub fn from_shared(mut src: Bytes) -> Result<Self, InvalidUriBytes> {
        let mut query = NONE;

        let mut i = 0;

        while i < src.len() {
            let b = src[i];

            match URI_CHARS[b as usize] {
                0 => {
                    if b == b'%' {
                        // Check that there are enough chars for a percent
                        // encoded char
                        let perc_encoded =
                            i + 3 <= src.len() && // enough capacity
                            HEX_DIGIT[src[i + 1] as usize] != 0 &&
                            HEX_DIGIT[src[i + 2] as usize] != 0;

                        if !perc_encoded {
                            return Err(ErrorKind::InvalidUriChar.into());
                        }

                        i += 3;
                        continue;
                    } else {
                        return Err(ErrorKind::InvalidUriChar.into());
                    }
                }
                b'?' => {
                    if query == NONE {
                        query = i as u16;
                    }
                }
                b'#' => {
                    // TODO: truncate
                    src.split_off(i);
                    break;
                }
                _ => {}
            }

            i += 1;
        }

        Ok(PathAndQuery {
            data: unsafe { ByteStr::from_utf8_unchecked(src) },
            query: query,
        })
    }

    fn empty() -> Self {
        PathAndQuery {
            data: ByteStr::new(),
            query: NONE,
        }
    }

    fn slash() -> Self {
        PathAndQuery {
            data: ByteStr::from_static("/"),
            query: NONE,
        }
    }

    fn star() -> Self {
        PathAndQuery {
            data: ByteStr::from_static("*"),
            query: NONE,
        }
    }

    /// Returns the path component
    ///
    /// The path component is **case sensitive**.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                                        |--------|
    ///                                             |
    ///                                           path
    /// ```
    ///
    /// If the URI is `*` then the path component is equal to `*`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::*;
    ///
    /// let path_and_query: PathAndQuery = "/hello/world".parse().unwrap();
    ///
    /// assert_eq!(path_and_query.path(), "/hello/world");
    /// ```
    #[inline]
    pub fn path(&self) -> &str {
        let ret = if self.query == NONE {
            &self.data[..]
        } else {
            &self.data[..self.query as usize]
        };

        if ret.is_empty() {
            return "/";
        }

        ret
    }

    /// Returns the query string component
    ///
    /// The query component contains non-hierarchical data that, along with data
    /// in the path component, serves to identify a resource within the scope of
    /// the URI's scheme and naming authority (if any). The query component is
    /// indicated by the first question mark ("?") character and terminated by a
    /// number sign ("#") character or by the end of the URI.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                                                   |-------------------|
    ///                                                             |
    ///                                                           query
    /// ```
    ///
    /// # Examples
    ///
    /// With a query string component
    ///
    /// ```
    /// # use http::uri::*;
    /// let path_and_query: PathAndQuery = "/hello/world?key=value&foo=bar".parse().unwrap();
    ///
    /// assert_eq!(path_and_query.query(), Some("key=value&foo=bar"));
    /// ```
    ///
    /// Without a query string component
    ///
    /// ```
    /// # use http::uri::*;
    /// let path_and_query: PathAndQuery = "/hello/world".parse().unwrap();
    ///
    /// assert!(path_and_query.query().is_none());
    /// ```
    #[inline]
    pub fn query(&self) -> Option<&str> {
        if self.query == NONE {
            None
        } else {
            let i = self.query + 1;
            Some(&self.data[i as usize..])
        }
    }

    /// Converts this `PathAndQuery` back to a sequence of bytes
    #[inline]
    pub fn into_bytes(self) -> Bytes {
        self.into()
    }
}

impl FromStr for PathAndQuery {
    type Err = InvalidUri;

    fn from_str(s: &str) -> Result<Self, InvalidUri> {
        PathAndQuery::from_shared(s.into()).map_err(|e| e.0)
    }
}

impl From<PathAndQuery> for Bytes {
    fn from(src: PathAndQuery) -> Bytes {
        src.data.into()
    }
}

impl fmt::Debug for PathAndQuery {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for PathAndQuery {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if !self.data.is_empty() {
            match self.data.as_bytes()[0] {
                b'/' | b'*' => write!(fmt, "{}", &self.data[..]),
                _ => write!(fmt, "/{}", &self.data[..]),
            }
        } else {
            write!(fmt, "/")
        }
    }
}

fn parse_full(mut s: Bytes) -> Result<Uri, InvalidUriBytes> {
    // Parse the scheme
    let scheme = match Scheme2::parse(&s[..]).map_err(InvalidUriBytes)? {
        Scheme2::None => Scheme2::None,
        Scheme2::Standard(p) => {
            // TODO: use truncate
            let _ = s.split_to(p.len() + 3);
            Scheme2::Standard(p)
        }
        Scheme2::Other(n) => {
            // Grab the protocol
            let mut scheme = s.split_to(n + 3);

            // Strip ://, TODO: truncate
            let _ = scheme.split_off(n);

            // Allocate the ByteStr
            let val = unsafe { ByteStr::from_utf8_unchecked(scheme) };

            Scheme2::Other(Box::new(val))
        }
    };

    // Find the end of the authority. The scheme will already have been
    // extracted.
    let authority_end = Authority::parse(&s[..]).map_err(InvalidUriBytes)?;

    if scheme.is_none() {
        if authority_end != s.len() {
            return Err(ErrorKind::InvalidFormat.into());
        }

        let authority = Authority {
            data: unsafe { ByteStr::from_utf8_unchecked(s) },
        };

        return Ok(Uri {
            scheme: scheme.into(),
            authority: authority,
            path_and_query: PathAndQuery::empty(),
        });
    }

    // Authority is required when absolute
    if authority_end == 0 {
        return Err(ErrorKind::InvalidFormat.into());
    }

    let authority = s.split_to(authority_end);
    let authority = Authority {
        data: unsafe { ByteStr::from_utf8_unchecked(authority) },
    };

    Ok(Uri {
        scheme: scheme.into(),
        authority: authority,
        path_and_query: PathAndQuery::from_shared(s)?,
    })
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


impl FromStr for Uri {
    type Err = InvalidUri;

    #[inline]
    fn from_str(s: &str) -> Result<Uri, InvalidUri> {
        Uri::from_shared(s.into()).map_err(|e| e.0)
    }
}

impl PartialEq for Uri {
    fn eq(&self, other: &Uri) -> bool {
        match (self.scheme(), other.scheme()) {
            (Some(a), Some(b)) => {
                if !a.eq_ignore_ascii_case(b) {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        };

        match (self.authority_part(), other.authority_part()) {
            (Some(a), Some(b)) => {
                if a != b {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        }

        if self.path() != other.path() {
            return false;
        }

        if self.query() != other.query() {
            return false;
        }

        true
    }
}

impl PartialEq<str> for Uri {
    fn eq(&self, other: &str) -> bool {
        let mut other = other.as_bytes();
        let mut absolute = false;

        if let Some(scheme) = self.scheme() {
            absolute = true;

            if other.len() < scheme.len() + 3 {
                return false;
            }

            if !scheme.as_bytes().eq_ignore_ascii_case(&other[..scheme.len()]) {
                return false;
            }

            other = &other[scheme.len()..];

            if &other[..3] != b"://" {
                return false;
            }

            other = &other[3..];
        }

        if let Some(auth) = self.authority_part() {
            let len = auth.data.len();
            absolute = true;

            if other.len() < len {
                return false;
            }

            if !auth.data.as_bytes().eq_ignore_ascii_case(&other[..len]) {
                return false;
            }

            other = &other[len..];
        }

        let path = self.path();

        if other.len() < path.len() || path.as_bytes() != &other[..path.len()] {
            if absolute && path == "/" {
                // PathAndQuery can be ommitted, fall through
            } else {
                return false;
            }
        } else {
            other = &other[path.len()..];
        }

        if let Some(query) = self.query() {
            if other[0] != b'?' {
                return false;
            }

            other = &other[1..];

            if other.len() < query.len() {
                return false;
            }

            if query.as_bytes() != &other[..query.len()] {
                return false;
            }

            other = &other[query.len()..];
        }

        other.is_empty() || other[0] == b'#'
    }
}

impl PartialEq<Uri> for str {
    fn eq(&self, uri: &Uri) -> bool {
        uri == self
    }
}

impl<'a> PartialEq<&'a str> for Uri {
    fn eq(&self, other: & &'a str) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<Uri> for &'a str {
    fn eq(&self, uri: &Uri) -> bool {
        uri == *self
    }
}

impl Eq for Uri {}

/// Returns a `Uri` representing `/`
impl Default for Uri {
    #[inline]
    fn default() -> Uri {
        Uri {
            scheme: Scheme::empty(),
            authority: Authority::empty(),
            path_and_query: PathAndQuery::slash(),
        }
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(scheme) = self.scheme() {
            write!(f, "{}://", scheme)?;
        }

        if let Some(authority) = self.authority_part() {
            write!(f, "{}", authority)?;
        }

        write!(f, "{}", self.path())?;

        if let Some(query) = self.query() {
            write!(f, "?{}", query)?;
        }

        Ok(())
    }
}

impl fmt::Debug for Uri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<ErrorKind> for InvalidUri {
    fn from(src: ErrorKind) -> InvalidUri {
        InvalidUri(src)
    }
}

impl From<ErrorKind> for InvalidUriBytes {
    fn from(src: ErrorKind) -> InvalidUriBytes {
        InvalidUriBytes(src.into())
    }
}

impl From<ErrorKind> for InvalidUriParts {
    fn from(src: ErrorKind) -> InvalidUriParts {
        InvalidUriParts(src.into())
    }
}

impl fmt::Display for InvalidUri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description().fmt(f)
    }
}

impl Error for InvalidUri {
    fn description(&self) -> &str {
        match self.0 {
            ErrorKind::InvalidUriChar => "invalid uri character",
            ErrorKind::InvalidScheme => "invalid scheme",
            ErrorKind::InvalidAuthority => "invalid authority",
            ErrorKind::InvalidFormat => "invalid format",
            ErrorKind::AuthorityMissing => "authority missing",
            ErrorKind::PathAndQueryMissing => "path missing",
            ErrorKind::TooLong => "uri too long",
            ErrorKind::Empty => "empty string",
            ErrorKind::SchemeTooLong => "scheme too long",
        }
    }
}

impl fmt::Display for InvalidUriBytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for InvalidUriParts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for InvalidUriBytes {
    fn description(&self) -> &str {
        self.0.description()
    }
}

impl Error for InvalidUriParts {
    fn description(&self) -> &str {
        self.0.description()
    }
}

impl<T> From<Protocol> for Scheme2<T> {
    fn from(src: Protocol) -> Self {
        Scheme2::Standard(src)
    }
}

impl From<Scheme2> for Scheme {
    fn from(src: Scheme2) -> Self {
        Scheme { inner: src }
    }
}

impl Hash for Uri {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        if let Some(scheme) = self.scheme() {
            scheme.as_bytes().to_ascii_lowercase().hash(state);
            "://".hash(state);
        }

        if let Some(auth) = self.authority_part() {
            auth.hash(state);
        }

        Hash::hash_slice(self.path().as_bytes(), state);

        if let Some(query) = self.query() {
            b'?'.hash(state);
            Hash::hash_slice(query.as_bytes(), state);
        }
    }
}

#[test]
fn test_char_table() {
    for (i, &v) in URI_CHARS.iter().enumerate() {
        if v != 0 {
            assert_eq!(i, v as usize);
        }
    }
}

macro_rules! test_parse {
    (
        $test_name:ident,
        $str:expr,
        $alt:expr,
        $($method:ident = $value:expr,)*
    ) => (
        #[test]
        fn $test_name() {
            let uri = Uri::from_str($str).unwrap();
            $(
            assert_eq!(uri.$method(), $value, stringify!($method));
            )+
            assert_eq!(uri, *$str);
            assert_eq!(uri, uri.clone());

            const ALT: &'static [&'static str] = &$alt;

            for &alt in ALT.iter() {
                let other: Uri = alt.parse().unwrap();
                assert_eq!(uri, *alt);
                assert_eq!(uri, other);
            }
        }
    );
}

test_parse! {
    test_uri_parse_path_and_query,
    "/some/path/here?and=then&hello#and-bye",
    [],

    scheme = None,
    authority_part = None,
    path = "/some/path/here",
    query = Some("and=then&hello"),
    host = None,
}

test_parse! {
    test_uri_parse_absolute_form,
    "http://127.0.0.1:61761/chunks",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1:61761".parse().unwrap()),
    path = "/chunks",
    query = None,
    host = Some("127.0.0.1"),
    port = Some(61761),
}

test_parse! {
    test_uri_parse_absolute_form_without_path,
    "https://127.0.0.1:61761",
    ["https://127.0.0.1:61761/"],

    scheme = Some("https"),
    authority_part = Some(&"127.0.0.1:61761".parse().unwrap()),
    path = "/",
    query = None,
    port = Some(61761),
    host = Some("127.0.0.1"),
}

test_parse! {
    test_uri_parse_asterisk_form,
    "*",
    [],

    scheme = None,
    authority_part = None,
    path = "*",
    query = None,
    host = None,
}

test_parse! {
    test_uri_parse_authority_no_port,
    "localhost",
    ["LOCALHOST", "LocaLHOSt"],

    scheme = None,
    authority_part = Some(&"localhost".parse().unwrap()),
    path = "",
    query = None,
    port = None,
    host = Some("localhost"),
}

test_parse! {
    test_uri_parse_authority_form,
    "localhost:3000",
    ["localhosT:3000"],

    scheme = None,
    authority_part = Some(&"localhost:3000".parse().unwrap()),
    path = "",
    query = None,
    host = Some("localhost"),
    port = Some(3000),
}

test_parse! {
    test_uri_parse_absolute_with_default_port_http,
    "http://127.0.0.1:80",
    ["http://127.0.0.1:80/"],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1:80".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = Some(80),
}

test_parse! {
    test_uri_parse_absolute_with_default_port_https,
    "https://127.0.0.1:443",
    ["https://127.0.0.1:443/"],

    scheme = Some("https"),
    authority_part = Some(&"127.0.0.1:443".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = Some(443),
}

test_parse! {
    test_uri_parse_fragment_questionmark,
    "http://127.0.0.1/#?",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_uri_parse_path_with_terminating_questionmark,
    "http://127.0.0.1/path?",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1".parse().unwrap()),
    path = "/path",
    query = Some(""),
    port = None,
}

test_parse! {
    test_uri_parse_absolute_form_with_empty_path_and_nonempty_query,
    "http://127.0.0.1?foo=bar",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1".parse().unwrap()),
    path = "/",
    query = Some("foo=bar"),
    port = None,
}

test_parse! {
    test_uri_parse_absolute_form_with_empty_path_and_fragment_with_slash,
    "http://127.0.0.1#foo/bar",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1".parse().unwrap()),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_uri_parse_absolute_form_with_empty_path_and_fragment_with_questionmark,
    "http://127.0.0.1#foo?bar",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1".parse().unwrap()),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_userinfo1,
    "http://a:b@127.0.0.1:1234/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"a:b@127.0.0.1:1234".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = Some(1234),
}

test_parse! {
    test_userinfo2,
    "http://a:b@127.0.0.1/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"a:b@127.0.0.1".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_userinfo3,
    "http://a@127.0.0.1/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"a@127.0.0.1".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_ipv6,
    "http://[2001:0db8:85a3:0000:0000:8a2e:0370:7334]/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"[2001:0db8:85a3:0000:0000:8a2e:0370:7334]".parse().unwrap()),
    host = Some("2001:0db8:85a3:0000:0000:8a2e:0370:7334"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_ipv6_shorthand,
    "http://[::1]/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"[::1]".parse().unwrap()),
    host = Some("::1"),
    path = "/",
    query = None,
    port = None,
}


test_parse! {
    test_ipv6_shorthand2,
    "http://[::]/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"[::]".parse().unwrap()),
    host = Some("::"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_ipv6_shorthand3,
    "http://[2001:db8::2:1]/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"[2001:db8::2:1]".parse().unwrap()),
    host = Some("2001:db8::2:1"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_ipv6_with_port,
    "http://[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:8008/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:8008".parse().unwrap()),
    host = Some("2001:0db8:85a3:0000:0000:8a2e:0370:7334"),
    path = "/",
    query = None,
    port = Some(8008),
}

test_parse! {
    test_percentage_encoded_path,
    "/echo/abcdefgh_i-j%20/abcdefg_i-j%20478",
    [],

    scheme = None,
    authority_part = None,
    host = None,
    path = "/echo/abcdefgh_i-j%20/abcdefg_i-j%20478",
    query = None,
    port = None,
}

#[test]
fn test_uri_parse_error() {
    fn err(s: &str) {
        Uri::from_str(s).unwrap_err();
    }

    err("http://");
    err("htt:p//host");
    err("hyper.rs/");
    err("hyper.rs?key=val");
    err("?key=val");
    err("localhost/");
    err("localhost?key=val");
    err("\0");
    err("http://[::1");
    err("http://::1]");
}

#[test]
fn test_max_uri_len() {
    let mut uri = vec![];
    uri.extend(b"http://localhost/");
    uri.extend(vec![b'a'; 70 * 1024]);

    let uri = String::from_utf8(uri).unwrap();
    let res: Result<Uri, InvalidUri> = uri.parse();

    assert_eq!(res.unwrap_err().0, ErrorKind::TooLong);
}

#[test]
fn test_long_scheme() {
    let mut uri = vec![];
    uri.extend(vec![b'a'; 256]);
    uri.extend(b"://localhost/");

    let uri = String::from_utf8(uri).unwrap();
    let res: Result<Uri, InvalidUri> = uri.parse();

    assert_eq!(res.unwrap_err().0, ErrorKind::SchemeTooLong);
}

#[test]
fn test_uri_to_path_and_query() {
    let cases = vec![
        ("/", "/"),
        ("/foo?bar", "/foo?bar"),
        ("/foo?bar#nope", "/foo?bar"),
        ("http://hyper.rs", "/"),
        ("http://hyper.rs/", "/"),
        ("http://hyper.rs/path", "/path"),
        ("http://hyper.rs?query", "/?query"),
        ("*", "*"),
    ];

    for case in cases {
        let uri = Uri::from_str(case.0).unwrap();
        let s = uri.path_and_query().unwrap().to_string();

        assert_eq!(s, case.1);
    }
}
