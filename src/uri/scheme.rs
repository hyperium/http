use std::convert::TryFrom;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use bytes::Bytes;

use super::{ErrorKind, InvalidUri};
use crate::byte_str::ByteStr;

/// Represents the scheme component of a URI
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Scheme(SchemeInner);

impl Scheme {
    /// HTTP protocol scheme
    pub const HTTP: Scheme = Self(SchemeInner::Http);

    /// HTTP protocol over TLS.
    pub const HTTPS: Scheme = Self(SchemeInner::Https);

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
        self.0.as_str()
    }

    pub(super) fn parse(bytes: &mut Bytes) -> Result<Option<Self>, InvalidUri> {
        let maybe_scheme = SchemeInner::parse(&bytes[..])?
            .map(|scheme| match scheme {
                SchemeInner::Http => {
                    // TODO: use truncate
                    let _ = bytes.split_to(7);
                    Self::HTTP
                }
                SchemeInner::Https => {
                    // TODO: use truncate
                    let _ = bytes.split_to(8);
                    Self::HTTPS
                }
                SchemeInner::Other(n) => {
                    // Grab the protocol
                    let mut scheme = bytes.split_to(n + 3);

                    // Strip ://, TODO: truncate
                    let _ = scheme.split_off(n);

                    // Allocate the ByteStr
                    let val = unsafe { ByteStr::from_utf8_unchecked(scheme) };

                    Self(SchemeInner::Other(Box::new(val)))
                }
            });
        Ok(maybe_scheme)
    }
}

impl AsRef<str> for Scheme {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Scheme {
    type Err = InvalidUri;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl TryFrom<&[u8]> for Scheme {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        s.try_into().map(Self)
    }
}

impl TryFrom<&str> for Scheme {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.try_into().map(Self)
    }
}

/// Case-insensitive equality
///
/// # Examples
///
/// ```
/// # use http::uri::Scheme;
/// let scheme: Scheme = "HTTP".parse().unwrap();
/// assert_eq!(scheme, *"http");
/// ```
impl PartialEq<str> for Scheme {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq_ignore_ascii_case(other)
    }
}

/// Case-insensitive equality
impl PartialEq<Scheme> for str {
    fn eq(&self, other: &Scheme) -> bool {
        other == self
    }
}

#[derive(Clone)]
enum SchemeInner<T = Box<ByteStr>> {
    Http,
    Https,
    Other(T),
}

impl SchemeInner {
    fn as_str(&self) -> &str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::Other(ref v) => &v[..],
        }
    }
}

impl SchemeInner<usize> {
    // Require the scheme to not be too long in order to enable further
    // optimizations later.
    const MAX_SCHEME_LEN: usize = 64;

    // scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    //
    // SCHEME_CHARS is a table of valid characters in the scheme part of a URI.  An
    // entry in the table is 0 for invalid characters. For valid characters the
    // entry is itself (i.e.  the entry for 43 is b'+' because b'+' == 43u8). An
    // important characteristic of this table is that all entries above 127 are
    // invalid. This makes all of the valid entries a valid single-byte UTF-8 code
    // point. This means that a slice of such valid entries is valid UTF-8.
    #[rustfmt::skip]
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

    fn parse(s: &[u8]) -> Result<Option<SchemeInner<usize>>, InvalidUri> {
        if s.len() <= 3 {
            // Too short for any non-empty scheme before `://`:
            Ok(None)
        } else if s.len() >= 7 && s[..7].eq_ignore_ascii_case(b"http://") {
            Ok(Some(SchemeInner::Http))
        } else if s.len() >= 8 && s[..8].eq_ignore_ascii_case(b"https://") {
            Ok(Some(SchemeInner::Https))
        } else {
            for i in 0..s.len() {
                let b = s[i];

                match Self::SCHEME_CHARS[b as usize] {
                    b':' => {
                        // Not enough data remaining
                        if s.len() < i + 3 {
                            break;
                        }

                        // Not a scheme
                        if &s[i + 1..i + 3] != b"//" {
                            break;
                        }

                        if i > Self::MAX_SCHEME_LEN {
                            return Err(ErrorKind::SchemeTooLong.into());
                        }

                        // Return scheme
                        return Ok(Some(SchemeInner::Other(i)));
                    }
                    // Invalid scheme character, abort
                    0 => break,
                    _ => {}
                }
            }
            Ok(None)
        }
    }

    // Postcondition: On all Ok() returns, s is valid UTF-8
    fn parse_exact(s: &[u8]) -> Result<SchemeInner<()>, InvalidUri> {
        if s.eq_ignore_ascii_case(b"http") {
            Ok(SchemeInner::Http)
        } else if s.eq_ignore_ascii_case(b"https") {
            Ok(SchemeInner::Https)
        } else if s.len() <= Self::MAX_SCHEME_LEN {
            // check that each byte in s is a SCHEME_CHARS which implies
            // that it is a valid single byte UTF-8 code point.
            for &b in s {
                match Self::SCHEME_CHARS[b as usize] {
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

            Ok(SchemeInner::Other(()))
        } else {
            Err(ErrorKind::SchemeTooLong.into())
        }
    }
}

impl fmt::Debug for SchemeInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for SchemeInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Eq for SchemeInner { }

/// Case-insensitive hashing
impl Hash for SchemeInner {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        match self {
            Self::Http => state.write_u8(1),
            Self::Https => state.write_u8(2),
            Self::Other(ref other) => {
                other.len().hash(state);
                for &b in other.as_bytes() {
                    state.write_u8(b.to_ascii_lowercase());
                }
            }
        }
    }
}

impl PartialEq for SchemeInner {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Http, Self::Http)
            | (Self::Https, Self::Https) => true,
            (Self::Other(a), Self::Other(b)) => a.eq_ignore_ascii_case(b),
            _ => false,
        }
    }
}

impl TryFrom<&[u8]> for SchemeInner {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        Ok(match SchemeInner::parse_exact(s)? {
            SchemeInner::Http => Self::Http,
            SchemeInner::Https => Self::Https,
            SchemeInner::Other(()) => {
                let bytes = Bytes::copy_from_slice(s);

                // Safety: postcondition on parse_exact() means that s and
                // hence bytes are valid UTF-8.
                let string = unsafe { ByteStr::from_utf8_unchecked(bytes) };

                Self::Other(Box::new(string))
            }
        })
    }
}

impl TryFrom<&str> for SchemeInner {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.as_bytes().try_into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn scheme_eq_to_str() {
        assert_eq!(&scheme("http"), "http");
        assert_eq!(&scheme("https"), "https");
        assert_eq!(&scheme("ftp"), "ftp");
        assert_eq!(&scheme("my+funky+scheme"), "my+funky+scheme");
    }

    #[test]
    fn standard_scheme_is_case_insensitive() {
        use std::collections::HashSet;

        for (s, expected) in [
            ("HTTP", Scheme::HTTP),
            ("hTtP", Scheme::HTTP),
            ("HTTPS", Scheme::HTTPS),
        ] {
            let parsed = scheme(s);
            assert_eq!(parsed, expected);
            assert_eq!(parsed.as_str(), expected.as_str());
            assert!(HashSet::from([expected]).contains(&parsed));
        }
    }

    #[test]
    fn invalid_scheme_is_error() {
        Scheme::try_from("my_funky_scheme").expect_err("Unexpectedly valid Scheme");

        // Invalid UTF-8
        Scheme::try_from([0xC0].as_ref()).expect_err("Unexpectedly valid Scheme");
    }

    fn scheme(s: &str) -> Scheme {
        s.parse().expect(&format!("Invalid scheme: {}", s))
    }
}
