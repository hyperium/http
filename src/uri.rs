//! URI component of request and response lines

use byte_str::ByteStr;

use bytes::Bytes;

use std::{fmt, u8, u16};
use std::ascii::AsciiExt;
use std::hash::{Hash, Hasher};
use std::str::{self, FromStr};

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
#[derive(Clone)]
pub struct Uri {
    scheme: Scheme,
    authority: Authority,
    origin_form: OriginForm,
}

/// Represents the scheme component of a URI
#[derive(Debug, Clone)]
pub struct Scheme {
    inner: Scheme2,
}

#[derive(Debug, Clone)]
enum Scheme2<T = Box<ByteStr>> {
    None,
    Standard(Protocol),
    Other(T),
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Protocol {
    Http,
    Https,
}

/// Represents the authority component of a URI.
#[derive(Debug, Clone)]
pub struct Authority {
    data: ByteStr,
}

/// Represents the path component of a URI
#[derive(Debug, Clone)]
pub struct OriginForm {
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
    pub origin_form: Option<OriginForm>,

    /// Allow extending in the future
    _priv: (),
}

/// An error resulting from a failed convertion of a URI from a &str.
#[derive(Debug)]
pub struct FromStrError(ErrorKind);

#[derive(Debug, Eq, PartialEq)]
enum ErrorKind {
    InvalidUriChar,
    InvalidScheme,
    InvalidAuthority,
    InvalidFormat,
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

impl Uri {
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
    /// let uri = Uri::try_from_shared(bytes).unwrap();
    ///
    /// assert_eq!(uri.host().unwrap(), "example.com");
    /// assert_eq!(uri.path(), "/foo");
    /// # }
    /// ```
    pub fn try_from_shared(s: Bytes) -> Result<Uri, FromStrError> {
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
                            origin_form: OriginForm::slash(),
                        });
                    }
                    b'*' => {
                        return Ok(Uri {
                            scheme: Scheme::empty(),
                            authority: Authority::empty(),
                            origin_form: OriginForm::star(),
                        });
                    }
                    _ => {
                        let authority = try!(Authority::try_from_shared(s));

                        return Ok(Uri {
                            scheme: Scheme::empty(),
                            authority: authority,
                            // TODO: Should this be empty instead?
                            origin_form: OriginForm::slash(),
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
                origin_form: try!(OriginForm::try_from_shared(s)),
            });
        }

        parse_full(s)
    }

    /// Returns the origin form component of the Uri
    ///
    /// This is the path and query string components or *.
    pub fn origin_form(&self) -> Option<&OriginForm> {
        if !self.scheme.inner.is_none() || self.authority.data.is_empty() {
            Some(&self.origin_form)
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
    pub fn path(&self) -> &str {
        if self.has_path() {
            self.origin_form.path()
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
    /// # Examples
    ///
    /// Absolute URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "http://example.org:80/hello/world".parse().unwrap();
    ///
    /// assert_eq!(uri.authority(), Some("example.org:80"));
    /// ```
    ///
    ///
    /// Relative URI
    ///
    /// ```
    /// # use http::Uri;
    /// let uri: Uri = "/hello/world".parse().unwrap();
    ///
    /// assert!(uri.authority().is_none());
    /// ```
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
    pub fn host(&self) -> Option<&str> {
        self.authority()
            .and_then(|a| a.split(":").next())
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
        self.authority()
            .and_then(|a| {
                a.find(":").and_then(|i| {
                    u16::from_str(&a[i+1..]).ok()
                })
            })
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
    pub fn query(&self) -> Option<&str> {
        self.origin_form.query()
    }

    fn has_path(&self) -> bool {
        !self.origin_form.data.is_empty() || !self.scheme.inner.is_none()
    }
}

impl From<Parts> for Uri {
    fn from(src: Parts) -> Self {
        if src.scheme.is_some() {
            assert!(src.authority.is_some(), "an authority must be provided if a scheme is provided");
            assert!(src.origin_form.is_some(), "an `OriginForm` must be provided if a scheme is provided");
        } else {
            if src.authority.is_some() {
                assert!(src.origin_form.is_none(), "`OriginForm` missing");
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

        let origin_form = match src.origin_form {
            Some(origin_form) => origin_form,
            None => OriginForm::empty(),
        };

        Uri {
            scheme: scheme,
            authority: authority,
            origin_form: origin_form,
        }
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
/// parts.origin_form = Some("/foo".parse().unwrap());
///
/// let uri = Uri::from(parts);
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
/// parts.origin_form = Some("/foo".parse().unwrap());
///
/// let uri = Uri::from(parts);
///
/// assert_eq!(uri.scheme().unwrap(), "http");
/// assert_eq!(uri.authority().unwrap(), "foo.com");
/// assert_eq!(uri.path(), "/foo");
/// ```
impl From<Uri> for Parts {
    fn from(src: Uri) -> Self {
        let origin_form = if src.has_path() {
            Some(src.origin_form)
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
            origin_form: origin_form,
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
    /// let scheme = Scheme::try_from_shared(bytes).unwrap();
    ///
    /// assert_eq!(scheme.as_str(), "http");
    /// # }
    /// ```
    pub fn try_from_shared(s: Bytes) -> Result<Self, FromStrError> {
        use self::Scheme2::*;

        match try!(Scheme2::parse_exact(&s[..])) {
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
}

impl FromStr for Scheme {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use self::Scheme2::*;

        match try!(Scheme2::parse_exact(s.as_bytes())) {
            None => Err(ErrorKind::InvalidScheme.into()),
            Standard(p) => Ok(Standard(p).into()),
            Other(_) => {
                Ok(Other(Box::new(s.into())).into())
            }
        }
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
    fn parse_exact(s: &[u8]) -> Result<Scheme2<()>, FromStrError> {
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

    fn parse(s: &[u8]) -> Result<Scheme2<usize>, FromStrError> {
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
    /// let authority = Authority::try_from_shared(bytes).unwrap();
    ///
    /// assert_eq!(authority.host(), "example.com");
    /// # }
    /// ```
    pub fn try_from_shared(s: Bytes) -> Result<Self, FromStrError> {
        let authority_end = try!(Authority::parse(&s[..]));

        if authority_end != s.len() {
            return Err(ErrorKind::InvalidUriChar.into());
        }

        Ok(Authority {
            data: unsafe { ByteStr::from_utf8_unchecked(s) },
        })
    }

    fn parse(s: &[u8]) -> Result<usize, FromStrError> {
        for (i, &b) in s.iter().enumerate() {
            match URI_CHARS[b as usize] {
                b'/' | b'?' | b'#' => {
                    return Ok(i);
                }
                0 => {
                    return Err(ErrorKind::InvalidUriChar.into());
                }
                _ => {}
            }
        }

        Ok(s.len())
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
    pub fn host(&self) -> &str {
        self.as_str().split(":").next().unwrap()
    }

    /// Return a str representation of the authority
    pub fn as_str(&self) -> &str {
        &self.data[..]
    }
}

impl FromStr for Authority {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, FromStrError> {
        let end = try!(Authority::parse(s.as_bytes()));

        if end != s.len() {
            return Err(ErrorKind::InvalidAuthority.into());
        }

        Ok(Authority { data: s.into() })
    }
}

impl OriginForm {
    /// Attempt to convert a `OriginForm` from `Bytes`.
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
    /// let origin_form = OriginForm::try_from_shared(bytes).unwrap();
    ///
    /// assert_eq!(origin_form.path(), "/hello");
    /// assert_eq!(origin_form.query(), Some("world"));
    /// # }
    /// ```
    pub fn try_from_shared(mut src: Bytes) -> Result<Self, FromStrError> {
        let mut query = NONE;

        for i in 0..src.len() {
            let b = src[i];

            match URI_CHARS[b as usize] {
                0 => {
                    return Err(ErrorKind::InvalidUriChar.into());
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
        }

        Ok(OriginForm {
            data: unsafe { ByteStr::from_utf8_unchecked(src) },
            query: query,
        })
    }

    fn empty() -> Self {
        OriginForm {
            data: ByteStr::new(),
            query: NONE,
        }
    }

    fn slash() -> Self {
        OriginForm {
            data: ByteStr::from_static("/"),
            query: NONE,
        }
    }

    fn star() -> Self {
        OriginForm {
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
    /// let origin_form: OriginForm = "/hello/world".parse().unwrap();
    ///
    /// assert_eq!(origin_form.path(), "/hello/world");
    /// ```
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
    /// let origin_form: OriginForm = "/hello/world?key=value&foo=bar".parse().unwrap();
    ///
    /// assert_eq!(origin_form.query(), Some("key=value&foo=bar"));
    /// ```
    ///
    /// Without a query string component
    ///
    /// ```
    /// # use http::uri::*;
    /// let origin_form: OriginForm = "/hello/world".parse().unwrap();
    ///
    /// assert!(origin_form.query().is_none());
    /// ```
    pub fn query(&self) -> Option<&str> {
        if self.query == NONE {
            None
        } else {
            let i = self.query + 1;
            Some(&self.data[i as usize..])
        }
    }
}

impl FromStr for OriginForm {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, FromStrError> {
        OriginForm::try_from_shared(s.into())
    }
}

impl fmt::Display for OriginForm {
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

fn parse_full(mut s: Bytes) -> Result<Uri, FromStrError> {
    // Parse the scheme
    let scheme = match try!(Scheme2::parse(&s[..])) {
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
    let authority_end = try!(Authority::parse(&s[..]));

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
            origin_form: OriginForm::empty(),
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
        origin_form: try!(OriginForm::try_from_shared(s)),
    })
}


impl FromStr for Uri {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Uri, FromStrError> {
        Uri::try_from_shared(s.into())
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

        match (self.authority(), other.authority()) {
            (Some(a), Some(b)) => {
                if !a.eq_ignore_ascii_case(b) {
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

        if let Some(auth) = self.authority() {
            absolute = true;

            if other.len() < auth.len() {
                return false;
            }

            if !auth.as_bytes().eq_ignore_ascii_case(&other[..auth.len()]) {
                return false;
            }

            other = &other[auth.len()..];
        }

        let path = self.path();

        if other.len() < path.len() || path.as_bytes() != &other[..path.len()] {
            if absolute && path == "/" {
                // OriginForm can be ommitted, fall through
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

impl Eq for Uri {}

/// Returns a `Uri` representing `/`
impl Default for Uri {
    fn default() -> Uri {
        Uri {
            scheme: Scheme::empty(),
            authority: Authority::empty(),
            origin_form: OriginForm::slash(),
        }
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(scheme) = self.scheme() {
            try!(write!(f, "{}://", scheme));
        }

        if let Some(authority) = self.authority() {
            try!(write!(f, "{}", authority));
        }

        try!(write!(f, "{}", self.path()));

        if let Some(query) = self.query() {
            try!(write!(f, "?{}", query));
        }

        Ok(())
    }
}

impl fmt::Debug for Uri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<ErrorKind> for FromStrError {
    fn from(src: ErrorKind) -> FromStrError {
        FromStrError(src)
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

        if let Some(auth) = self.authority() {
            auth.as_bytes().to_ascii_lowercase().hash(state);
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
            assert_eq!(uri.$method(), $value);
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
    test_uri_parse_origin_form,
    "/some/path/here?and=then&hello#and-bye",
    [],

    scheme = None,
    authority = None,
    path = "/some/path/here",
    query = Some("and=then&hello"),
    host = None,
}

test_parse! {
    test_uri_parse_absolute_form,
    "http://127.0.0.1:61761/chunks",
    [],

    scheme = Some("http"),
    authority = Some("127.0.0.1:61761"),
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
    authority = Some("127.0.0.1:61761"),
    path = "/",
    query = None,
    port = Some(61761),
}

test_parse! {
    test_uri_parse_asterisk_form,
    "*",
    [],

    scheme = None,
    authority = None,
    path = "*",
    query = None,
}

test_parse! {
    test_uri_parse_authority_no_port,
    "localhost",
    ["LOCALHOST", "LocaLHOSt"],

    scheme = None,
    authority = Some("localhost"),
    path = "",
    query = None,
    port = None,
}

test_parse! {
    test_uri_parse_authority_form,
    "localhost:3000",
    ["localhosT:3000"],

    scheme = None,
    authority = Some("localhost:3000"),
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
    authority = Some("127.0.0.1:80"),
    path = "/",
    query = None,
    port = Some(80),
}

test_parse! {
    test_uri_parse_absolute_with_default_port_https,
    "https://127.0.0.1:443",
    ["https://127.0.0.1:443/"],

    scheme = Some("https"),
    authority = Some("127.0.0.1:443"),
    path = "/",
    query = None,
    port = Some(443),
}

test_parse! {
    test_uri_parse_fragment_questionmark,
    "http://127.0.0.1/#?",
    [],

    scheme = Some("http"),
    authority = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_uri_parse_path_with_terminating_questionmark,
    "http://127.0.0.1/path?",
    [],

    scheme = Some("http"),
    authority = Some("127.0.0.1"),
    path = "/path",
    query = Some(""),
    port = None,
}

test_parse! {
    test_uri_parse_absolute_form_with_empty_path_and_nonempty_query,
    "http://127.0.0.1?foo=bar",
    [],

    scheme = Some("http"),
    authority = Some("127.0.0.1"),
    path = "/",
    query = Some("foo=bar"),
    port = None,
}

test_parse! {
    test_uri_parse_absolute_form_with_empty_path_and_fragment_with_slash,
    "http://127.0.0.1#foo/bar",
    [],

    scheme = Some("http"),
    authority = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_uri_parse_absolute_form_with_empty_path_and_fragment_with_questionmark,
    "http://127.0.0.1#foo?bar",
    [],

    scheme = Some("http"),
    authority = Some("127.0.0.1"),
    path = "/",
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
}

#[test]
fn test_max_uri_len() {
    let mut uri = vec![];
    uri.extend(b"http://localhost/");
    uri.extend(vec![b'a'; 70 * 1024]);

    let uri = String::from_utf8(uri).unwrap();
    let res: Result<Uri, FromStrError> = uri.parse();

    assert_eq!(res.unwrap_err().0, ErrorKind::TooLong);
}

#[test]
fn test_long_scheme() {
    let mut uri = vec![];
    uri.extend(vec![b'a'; 256]);
    uri.extend(b"://localhost/");

    let uri = String::from_utf8(uri).unwrap();
    let res: Result<Uri, FromStrError> = uri.parse();

    assert_eq!(res.unwrap_err().0, ErrorKind::SchemeTooLong);
}

#[test]
fn test_uri_to_origin_form() {
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
        let s = uri.origin_form().unwrap().to_string();

        assert_eq!(s, case.1);
    }
}
