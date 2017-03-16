//! The HTTP request method

use self::Inner::*;
use str::Str;

use std::fmt;
use std::convert::AsRef;

/// The Request Method (VERB)
///
/// Currently includes 8 variants representing the 8 methods defined in
/// [RFC 7230](https://tools.ietf.org/html/rfc7231#section-4.1), plus PATCH,
/// and an Extension variant for all extensions.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Method(Inner);

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
    Extension(Str)
}

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
    /// Return a `Method` instance representing a non-standard HTTP method.
    ///
    /// # Examples
    ///
    /// ```
    /// use http::Method;
    ///
    /// let method = Method::extension("FOO");
    /// assert_eq!(method, "FOO");
    /// ```
    pub fn extension(s: &str) -> Method {
        Method(Extension(s.into()))
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
            Extension(ref s) => s.as_ref()
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
        fmt.write_str(match self.0 {
            Options => "OPTIONS",
            Get => "GET",
            Post => "POST",
            Put => "PUT",
            Delete => "DELETE",
            Head => "HEAD",
            Trace => "TRACE",
            Connect => "CONNECT",
            Patch => "PATCH",
            Extension(ref s) => s.as_ref()
        })
    }
}

impl Default for Method {
    fn default() -> Method {
        GET
    }
}
