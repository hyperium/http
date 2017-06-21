//! HTTP response types.

use header::{HeaderMap, HeaderValue};
use status::StatusCode;
use version::Version;

/// Represents an HTTP response
///
/// An HTTP response consists of a head and a potentially optional body. The body
/// component is generic, enabling arbitrary types to represent the HTTP body.
/// For example, the body could be `Vec<u8>`, a `Stream` of byte chunks, or a
/// value that has been deserialized.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Response<T> {
    head: Head,
    body: T,
}

/// An HTTP response head
///
/// The HTTP response head consists of a status, version, and a set of
/// header fields.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Head {
    /// The response's status
    pub status: StatusCode,

    /// The response's version
    pub version: Version,

    /// The response's headers
    pub headers: HeaderMap<HeaderValue>,

    _priv: (),
}

impl<T> Response<T> {
    /// Creates a new `Response` with the given head and body
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let head = response::Head::default();
    /// let response = Response::from_parts(head, "hello world");
    ///
    /// assert_eq!(response.status(), status::OK);
    /// assert_eq!(*response.body(), "hello world");
    /// ```
    pub fn from_parts(head: Head, body: T) -> Response<T> {
        Response {
            head: head,
            body: body,
        }
    }

    /// Returns the `StatusCode`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<()> = Response::default();
    /// assert_eq!(response.status(), status::OK);
    /// ```
    pub fn status(&self) -> StatusCode {
        self.head.status
    }

    /// Returns a mutable reference to the associated `StatusCode`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut response: Response<()> = Response::default();
    /// *response.status_mut() = status::CREATED;
    /// assert_eq!(response.status(), status::CREATED);
    /// ```
    pub fn status_mut(&mut self) -> &mut StatusCode {
        &mut self.head.status
    }

    /// Returns a reference to the associated version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<()> = Response::default();
    /// assert_eq!(response.version(), version::HTTP_11);
    /// ```
    pub fn version(&self) -> Version {
        self.head.version
    }

    /// Returns a mutable reference to the associated version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut response: Response<()> = Response::default();
    /// *response.version_mut() = version::HTTP_2;
    /// assert_eq!(response.version(), version::HTTP_2);
    /// ```
    pub fn version_mut(&mut self) -> &mut Version {
        &mut self.head.version
    }

    /// Returns a reference to the associated header field map.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<()> = Response::default();
    /// assert!(response.headers().is_empty());
    /// ```
    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.head.headers
    }

    /// Returns a mutable reference to the associated header field map.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::*;
    /// let mut response: Response<()> = Response::default();
    /// response.headers_mut().insert("hello", HeaderValue::from_static("world"));
    /// assert!(!response.headers().is_empty());
    /// ```
    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        &mut self.head.headers
    }

    /// Returns a reference to the associated HTTP body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<String> = Response::default();
    /// assert!(response.body().is_empty());
    /// ```
    pub fn body(&self) -> &T {
        &self.body
    }

    /// Returns a mutable reference to the associated HTTP body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut response: Response<String> = Response::default();
    /// response.body_mut().push_str("hello world");
    /// assert!(!response.body().is_empty());
    /// ```
    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }

    /// Consumes the response returning the head and body parts.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<()> = Response::default();
    /// let (head, body) = response.into_parts();
    /// let response::Head { status, .. } = head;
    /// assert_eq!(status, status::OK);
    /// ```
    pub fn into_parts(self) -> (Head, T) {
        (self.head, self.body)
    }
}

impl<T: Default> From<Head> for Response<T> {
    fn from(src: Head) -> Response<T> {
        Response {
            head: src,
            body: T::default(),
        }
    }
}
