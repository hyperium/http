//! The HTTP request method

use self::Inner::*;

use std::{fmt, str};
use std::convert::AsRef;
use std::error::Error;

/// The Request Method (VERB)
///
/// Currently includes 8 variants representing the 8 methods defined in
/// [RFC 7230](https://tools.ietf.org/html/rfc7231#section-4.1), plus PATCH,
/// and an Extension variant for all extensions.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Method(Inner);

#[derive(Debug)]
pub struct FromBytesError;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Inner {
    Options,
    Get,
    Post,
    Put,
    Delete,
    Head,
    Trace,
    Connect,
    Patch,
    // If the extension is short enough, store it inline
    ExtensionInline([u8; MAX_INLINE], u8),
    // Otherwise, allocate it
    ExtensionAllocated(Box<[u8]>),
}

const MAX_INLINE: usize = 15;

// From the HTTP spec section 5.1.1, the HTTP method is case-sensitive and can
// contain the following characters:
//
// ```
// method = token
// token = 1*tchar
// tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
//     "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
// ```
//
// https://www.w3.org/Protocols/HTTP/1.1/draft-ietf-http-v11-spec-01#Method
//
const METHOD_CHARS: [u8; 256] = [
    //  0      1      2      3      4      5      6      7      8      9
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', //   x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', //  1x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', //  2x
    b'\0', b'\0', b'\0',  b'!', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', //  3x
    b'\0', b'\0',  b'*',  b'+', b'\0',  b'-',  b'.', b'\0',  b'0',  b'1', //  4x
     b'2',  b'3',  b'4',  b'5',  b'6',  b'7',  b'8',  b'9', b'\0', b'\0', //  5x
    b'\0', b'\0', b'\0', b'\0', b'\0',  b'A',  b'B',  b'C',  b'D',  b'E', //  6x
     b'F',  b'G',  b'H',  b'I',  b'J',  b'K',  b'L',  b'M',  b'N',  b'O', //  7x
     b'P',  b'Q',  b'R',  b'S',  b'T',  b'U',  b'V',  b'W',  b'X',  b'Y', //  8x
     b'Z', b'\0', b'\0', b'\0',  b'^',  b'_',  b'`',  b'a',  b'b',  b'c', //  9x
     b'd',  b'e',  b'f',  b'g',  b'h',  b'i',  b'j',  b'k',  b'l',  b'm', // 10x
     b'n',  b'o',  b'p',  b'q',  b'r',  b's',  b't',  b'u',  b'v',  b'w', // 11x
     b'x',  b'y',  b'z', b'\0',  b'|', b'\0',  b'~', b'\0', b'\0', b'\0', // 12x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 13x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 14x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 15x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 16x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 17x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 18x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 19x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 20x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 21x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 22x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 23x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', // 24x
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0'                              // 25x
];

/// GET
pub const GET: Method = Method(Get);

/// POST
pub const POST: Method = Method(Post);

/// PUT
pub const PUT: Method = Method(Put);

/// DELETE
pub const DELETE: Method = Method(Delete);

/// HEAD
pub const HEAD: Method = Method(Head);

/// OPTIONS
pub const OPTIONS: Method = Method(Options);

/// CONNECT
pub const CONNECT: Method = Method(Connect);

/// PATCH
pub const PATCH: Method = Method(Patch);

/// TRACE
pub const TRACE: Method = Method(Trace);

impl Method {
    /// Converts a slice of bytes to an HTTP method.
    pub fn from_bytes(src: &[u8]) -> Result<Method, FromBytesError> {
        match src.len() {
            3 => {
                match src {
                    b"GET" => Ok(Method(Get)),
                    b"PUT" => Ok(Method(Put)),
                    _ => Method::extension_inline_checked(src),
                }
            }
            4 => {
                match src {
                    b"POST" => Ok(Method(Post)),
                    b"HEAD" => Ok(Method(Head)),
                    _ => Method::extension_inline_checked(src),
                }
            }
            5 => {
                match src {
                    b"PATCH" => Ok(Method(Patch)),
                    b"TRACE" => Ok(Method(Trace)),
                    _ => Method::extension_inline_checked(src),
                }
            }
            6 => {
                match src {
                    b"DELETE" => Ok(Method(Delete)),
                    _ => Method::extension_inline_checked(src),
                }
            }
            7 => {
                match src {
                    b"OPTIONS" => Ok(Method(Options)),
                    b"CONNECT" => Ok(Method(Connect)),
                    _ => Method::extension_inline_checked(src),
                }
            }
            _ => {
                if src.len() < MAX_INLINE {
                    Method::extension_inline_checked(src)
                } else {
                    Method::extension_allocated_checked(src)
                }
            }
        }
    }

    /// Converts a slice of bytes to an HTTP method without validating the input
    /// data.
    ///
    /// The caller must ensure that the input is a valid HTTP method (see HTTP
    /// spec section 5.1.1) and that the method is **not** a standard HTTP
    /// method, i.e. one that is defined as a constant in this module.
    ///
    /// The function is unsafe as the input is not checked as valid UTF-8.
    pub unsafe fn from_bytes_unchecked(src: &[u8]) -> Method {
        if src.len() < MAX_INLINE {
            let mut data: [u8; MAX_INLINE] = Default::default();

            data[0..src.len()].copy_from_slice(src);

            Method(ExtensionInline(data, src.len() as u8))
        } else {
            let mut data = vec![];
            data.extend(src);

            Method(ExtensionAllocated(data.into_boxed_slice()))
        }
    }

    fn extension_inline_checked(src: &[u8]) -> Result<Method, FromBytesError> {
        let mut data: [u8; MAX_INLINE] = Default::default();

        try!(write_checked(src, &mut data));

        Ok(Method(ExtensionInline(data, src.len() as u8)))
    }

    fn extension_allocated_checked(src: &[u8]) -> Result<Method, FromBytesError> {
        let mut data: Vec<u8> = vec![0; src.len()];

        try!(write_checked(src, &mut data));

        Ok(Method(ExtensionAllocated(data.into_boxed_slice())))
    }

    /// Whether a method is considered "safe", meaning the request is
    /// essentially read-only.
    ///
    /// See [the spec](https://tools.ietf.org/html/rfc7231#section-4.2.1)
    /// for more words.
    pub fn is_safe(&self) -> bool {
        match self.0 {
            Get | Head | Options | Trace => true,
            _ => false
        }
    }

    /// Whether a method is considered "idempotent", meaning the request has
    /// the same result if executed multiple times.
    ///
    /// See [the spec](https://tools.ietf.org/html/rfc7231#section-4.2.2) for
    /// more words.
    pub fn is_idempotent(&self) -> bool {
        if self.is_safe() {
            true
        } else {
            match self.0 {
                Put | Delete => true,
                _ => false
            }
        }
    }
}

fn write_checked(src: &[u8], dst: &mut [u8]) -> Result<(), FromBytesError> {
    for (i, &b) in src.iter().enumerate() {
        let b = METHOD_CHARS[b as usize];

        if b == 0 {
            return Err(FromBytesError);
        }

        dst[i] = b;
    }

    Ok(())
}

impl AsRef<str> for Method {
    fn as_ref(&self) -> &str {
        match self.0 {
            Options => "OPTIONS",
            Get => "GET",
            Post => "POST",
            Put => "PUT",
            Delete => "DELETE",
            Head => "HEAD",
            Trace => "TRACE",
            Connect => "CONNECT",
            Patch => "PATCH",
            ExtensionInline(ref data, len) => {
                unsafe {
                    str::from_utf8_unchecked(&data[..len as usize])
                }
            }
            ExtensionAllocated(ref data) => {
                unsafe {
                    str::from_utf8_unchecked(data)
                }
            }
        }
    }
}

impl PartialEq<str> for Method {
    fn eq(&self, other: &str) -> bool {
        self.as_ref() == other
    }
}

impl<'a> PartialEq<&'a str> for Method {
    fn eq(&self, other: &&'a str) -> bool {
        self.as_ref() == *other
    }
}

impl fmt::Display for Method {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.as_ref())
    }
}

impl Default for Method {
    fn default() -> Method {
        GET
    }
}

impl fmt::Display for FromBytesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for FromBytesError {
    fn description(&self) -> &str {
        "invalid HTTP method"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}
