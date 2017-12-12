use std::fmt;
use std::str::FromStr;

use bytes::Bytes;

use byte_str::ByteStr;
use super::{ErrorKind, InvalidUri, InvalidUriBytes, URI_CHARS};

/// Represents the path component of a URI
#[derive(Clone)]
pub struct PathAndQuery {
    pub(super) data: ByteStr,
    pub(super) query: u16,
}

const NONE: u16 = ::std::u16::MAX;

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

    /// Convert a `PathAndQuery` from a static string.
    ///
    /// This function will not perform any copying, however the string is
    /// checked to ensure that it is valid.
    ///
    /// # Panics
    ///
    /// This function panics if the argument is an invalid path and query.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::*;
    /// let v = PathAndQuery::from_static("/hello?world");
    ///
    /// assert_eq!(v.path(), "/hello");
    /// assert_eq!(v.query(), Some("world"));
    /// ```
    #[inline]
    pub fn from_static(src: &'static str) -> Self {
        let src = Bytes::from_static(src.as_bytes());

        PathAndQuery::from_shared(src)
            .unwrap()
    }

    pub(super) fn empty() -> Self {
        PathAndQuery {
            data: ByteStr::new(),
            query: NONE,
        }
    }

    pub(super) fn slash() -> Self {
        PathAndQuery {
            data: ByteStr::from_static("/"),
            query: NONE,
        }
    }

    pub(super) fn star() -> Self {
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
