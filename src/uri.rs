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
    data: Bytes,
    marks: Marks,
}

/// An error resulting from a failed convertion of a URI from a &str.
#[derive(Debug)]
pub struct FromStrError(ErrorKind);

#[derive(Clone)]
struct Marks {
    scheme: Scheme,
    authority_end: u16,
    query: u16,
    fragment: u16,
}

#[derive(Clone, Eq, PartialEq)]
enum Scheme {
    None,
    Http,
    Https,
    Other(u8),
}

#[derive(Debug, Eq, PartialEq)]
enum ErrorKind {
    InvalidUriChar,
    InvalidFormat,
    TooLong,
    Empty,
    SchemeTooLong,
}

// u16::MAX is reserved for None
const MAX_LEN: usize = (u16::MAX - 1) as usize;
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
    /// Get the path of this `Uri`.
    pub fn path(&self) -> &str {
        let start = self.marks.path_start();

        let end = self.marks.query_start()
            .or(self.marks.fragment_start())
            .unwrap_or(self.data.len());

        debug_assert!(end >= start);

        let ret = unsafe {
            str::from_utf8_unchecked(&self.data[start..end])
        };

        if ret.is_empty() {
            if self.scheme().is_some() {
                return "/";
            }
        }

        ret
    }

    /// Get the scheme of this `Uri`.
    pub fn scheme(&self) -> Option<&str> {
        match self.marks.scheme {
            Scheme::Http => Some("http"),
            Scheme::Https => Some("https"),
            Scheme::None => None,
            Scheme::Other(end) => {
                unsafe {
                    Some(str::from_utf8_unchecked(&self.data[..end as usize]))
                }
            }
        }
    }

    /// Get the authority of this `Uri`.
    pub fn authority(&self) -> Option<&str> {
        if let Some(end) = self.marks.authority_end() {
            let start = match self.marks.scheme {
                Scheme::Other(len) => len as usize + 3,
                _ => 0,
            };

            let ret = unsafe {
                str::from_utf8_unchecked(&self.data[start..end])
            };

            Some(ret)
        } else {
            None
        }
    }

    /// Get the host of this `Uri`.
    pub fn host(&self) -> Option<&str> {
        self.authority()
            .and_then(|a| a.split(":").next())
    }

    /// Get the port of this `Uri`.
    pub fn port(&self) -> Option<u16> {
        self.authority()
            .and_then(|a| {
                a.find(":").and_then(|i| {
                    u16::from_str(&a[i+1..]).ok()
                })
            })
    }

    /// Get the query string of this `Uri`, starting after the `?`.
    pub fn query(&self) -> Option<&str> {
        let start = self.marks.query;

        if start == NONE {
            return None;
        }

        let mut end = self.marks.fragment as usize;

        if end == NONE as usize {
            end = self.data.len();
        }

        let ret = unsafe {
            str::from_utf8_unchecked(&self.data[(start+1) as usize..end])
        };

        Some(ret)
    }

    fn fragment(&self) -> Option<&str> {
        let start = self.marks.fragment;

        if start == NONE {
            return None;
        }

        let ret = unsafe {
            str::from_utf8_unchecked(&self.data[(start+1) as usize..])
        };

        Some(ret)
    }
}

impl Marks {
    fn authority_end(&self) -> Option<usize> {
        if self.authority_end == NONE {
            None
        } else {
            Some(self.authority_end as usize)
        }
    }

    fn path_start(&self) -> usize {
        self.authority_end()
            .unwrap_or_else(|| {
                match self.scheme {
                    Scheme::Other(len) => len as usize + 3,
                    _ => 0,
                }
            })
    }

    fn query_start(&self) -> Option<usize> {
        if self.query == NONE {
            None
        } else {
            Some(self.query as usize)
        }
    }

    fn fragment_start(&self) -> Option<usize> {
        if self.fragment == NONE {
            None
        } else {
            Some(self.fragment as usize)
        }
    }
}

/// Parse a string into a `Uri`.
fn parse(s: &[u8]) -> Result<Marks, ErrorKind> {
    use self::ErrorKind::*;

    if s.len() > MAX_LEN {
        return Err(TooLong);
    }

    if s.len() == 0 {
        return Err(Empty);
    }

    match s.len() {
        0 => {
            return Err(Empty);
        }
        1 => {
            match s[0] {
                b'/' | b'*'=> {
                    return Ok(Marks {
                        scheme: Scheme::None,
                        authority_end: NONE,
                        query: NONE,
                        fragment: NONE,
                    });
                }
                _ => {
                    return Ok(Marks {
                        scheme: Scheme::None,
                        authority_end: 1,
                        query: NONE,
                        fragment: NONE,
                    });
                }
            }
        }
        _ => {}
    }

    if s[0] == b'/' {
        let (query, fragment) = try!(parse_query(s, 0));

        return Ok(Marks {
            scheme: Scheme::None,
            authority_end: NONE,
            query: query,
            fragment: fragment,
        });
    }

    parse_full(s)
}

fn parse_scheme(s: &[u8]) -> Result<(Scheme, usize, &[u8]), ErrorKind> {
    if s.len() >= 7 {
        // Check for HTTP
        if s[..7].eq_ignore_ascii_case(b"http://") {
            // Prefix will be striped
            return Ok((Scheme::Http, 0, &s[7..]));
        }
    }

    if s.len() >= 8 {
        // Check for HTTPs
        // Check for HTTP
        if s[..8].eq_ignore_ascii_case(b"http://") {
            return Ok((Scheme::Https, 0, &s[8..]));
        }
    }

    if s.len() > 3 {
        for (i, &b) in s.iter().enumerate() {
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

                    if i > u8::MAX as usize {
                        return Err(ErrorKind::SchemeTooLong);
                    }

                    return Ok((Scheme::Other(i as u8), i + 3, s));
                }
                // Invald scheme character, abort
                0 => break,
                _ => {}
            }
        }
    }

    Ok((Scheme::None, 0, s))
}

fn parse_authority(s: &[u8], pos: usize) -> Result<u16, ErrorKind> {
    for (i, &b) in s[pos..].iter().enumerate() {
        match URI_CHARS[b as usize] {
            b'/' => {
                return Ok((pos + i) as u16);
            }
            0 => {
                return Err(ErrorKind::InvalidUriChar);
            }
            _ => {}
        }
    }

    Ok(s.len() as u16)
}

fn parse_full(s: &[u8]) -> Result<Marks, ErrorKind> {
    let (scheme, pos, s) = try!(parse_scheme(s));
    let authority = try!(parse_authority(s, pos));

    if scheme == Scheme::None {
        if authority as usize != s.len() {
            return Err(ErrorKind::InvalidFormat);
        }

        return Ok(Marks {
            scheme: scheme,
            authority_end: authority,
            query: NONE,
            fragment: NONE,
        });
    }

    let (query, fragment) = try!(parse_query(s, authority as usize));

    Ok(Marks {
        scheme: scheme,
        authority_end: authority,
        query: query,
        fragment: fragment,
    })
}

fn parse_query(s: &[u8], pos: usize) -> Result<(u16, u16), ErrorKind> {
    let mut query = NONE;
    let mut fragment = NONE;

    for (i, &b) in s[pos..].iter().enumerate() {
        match URI_CHARS[b as usize] {
            0 => {
                return Err(ErrorKind::InvalidUriChar);
            }
            b'?' => {
                if query == NONE {
                    if fragment == NONE {
                        query = (pos + i) as u16;
                    }
                }
            }
            b'#' => {
                if fragment == NONE {
                    fragment = (pos + i) as u16;
                }
            }
            _ => {}
        }
    }

    Ok((query, fragment))
}

impl FromStr for Uri {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Uri, FromStrError> {
        let marks = try!(parse(s.as_bytes()));

        let data = match marks.scheme {
            Scheme::None | Scheme::Other(..) => Bytes::from(s),
            Scheme::Http => Bytes::from(&s.as_bytes()[7..]),
            Scheme::Https => Bytes::from(&s.as_bytes()[8..]),
        };

        Ok(Uri {
            data: data,
            marks: marks,
        })
    }
}

impl PartialEq for Uri {
    fn eq(&self, other: &Uri) -> bool {
        let m = match (self.scheme(), other.scheme()) {
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            (None, None) => true,
            _ => false,
        };

        if !m {
            return false;
        }

        let m = match (self.authority(), other.authority()) {
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            (None, None) => true,
            _ => false,
        };

        if !m {
            return false;
        }

        if self.path() != other.path() {
            return false;
        }

        if self.query() != other.query() {
            return false;
        }

        if self.fragment() != other.fragment() {
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
                // Path can be ommitted, fall through
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

        if let Some(frag) = self.fragment() {
            if other[0] != b'#' {
                return false;
            }

            other = &other[1..];

            if other.len() < frag.len() {
                return false;
            }

            if frag.as_bytes() != &other[..frag.len()] {
                return false;
            }

            other = &other[frag.len()..];
        }

        other.is_empty()
    }
}

impl Eq for Uri {}

impl Default for Uri {
    fn default() -> Uri {
        Uri {
            data: Bytes::from_static(b"/"),
            marks: Marks {
                scheme: Scheme::None,
                authority_end: NONE,
                query: NONE,
                fragment: NONE,
            }
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

        if let Some(fragment) = self.fragment() {
            try!(write!(f, "#{}", fragment));
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

        if let Some(fragment) = self.fragment() {
            b'#'.hash(state);
            Hash::hash_slice(fragment.as_bytes(), state);
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
    fragment = Some("and-bye"),
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
    fragment = None,
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
    fragment = None,
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
    fragment = None,
}

test_parse! {
    test_uri_parse_authority_no_port,
    "localhost",
    ["LOCALHOST", "LocaLHOSt"],

    scheme = None,
    authority = Some("localhost"),
    path = "",
    query = None,
    fragment = None,
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
    fragment = None,
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
    fragment = None,
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
    fragment = None,
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
    fragment = Some("?"),
    port = None,
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
