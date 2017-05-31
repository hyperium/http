//! HTTP request types.

use Uri;
use header::{HeaderMap, HeaderValue};
use method::Method;
use version::Version;

/// Represents an HTTP request.
///
/// An HTTP request consists of a head and a potentially optional body. The body
/// component is generic, enabling arbitrary types to represent the HTTP body.
/// For example, the body could be `Vec<u8>`, a `Stream` of byte chunks, or a
/// value that has been deserialized.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Request<T> {
    head: Head,
    body: T,
}

/// An HTTP request head
///
/// The HTTP request head consists of a method, uri, version, and a set of
/// header fields.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Head {
    pub method: Method,
    pub uri: Uri,
    pub version: Version,
    pub headers: HeaderMap<HeaderValue>,
    _priv: (),
}

impl<T> Request<T> {
    /// Creates a new `Request` with the given head and body
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let head = request::Head::new(
    ///     method::GET,
    ///     "/".parse().unwrap(),
    ///     version::HTTP_11,
    ///     HeaderMap::new());
    ///
    /// let request = Request::new(head, "hello world");
    ///
    /// assert_eq!(*request.method(), method::GET);
    /// assert_eq!(*request.body(), "hello world");
    /// ```
    pub fn from_parts(head: Head, body: T) -> Request<T> {
        Request {
            head: head,
            body: body,
        }
    }

    /// Returns a reference to the associated HTTP method.
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn method(&self) -> &Method {
        &self.head.method
    }

    /// Returns a mutable reference to the associated HTTP method.
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.head.method
    }

    /// Returns a reference to the associated URI.
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn uri(&self) -> &Uri {
        &self.head.uri
    }

    /// Returns a mutable reference to the associated URI.
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn uri_mut(&self) -> &mut Uri {
        &mut self.head.uri
    }

    /// Returns a reference to the associated version.
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn version(&self) -> &Version {
        &self.head.version
    }

    /// Returns a mutable reference to the associated version.
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn version_mut(&mut self) -> &mut Version {
        &self.head.version
    }

    /// Returns a reference to the associated header field map.
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.head.headers
    }

    /// Returns a mutable reference to the associated header field map.
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        &mut self.head.headers
    }

    /// Returns a reference to the associated HTTP body.
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn body(&self) -> &T {
        &self.body
    }

    /// Returns a mutable reference to the associated HTTP body.
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }

    /// Consumes the request returning the head and body parts.
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn into_parts(self) -> (Head, T) {
        (self.head, self.body)
    }
}

impl Head {
    /// Creates a new `Head` with the given çomponents
    ///
    /// # Examples
    ///
    /// ```
    /// unimplemented!();
    /// ```
    pub fn new(method: Method,
               uri: Uri,
               version: Version,
               headers: HeaderMap<HeaderValue>) -> Head
    {
        Head {
            method: method,
            uri: uri,
            version: version,
            headers: headers,
            _priv: (),
        }
    }
}

impl<T: Default> From<Head> for Request<T> {
    fn from(src: Head) -> Request<T> {
        Request {
            head: src,
            body: T::default(),
        }
    }
}
