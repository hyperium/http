//! HTTP request types.
//!
//! This module contains structs related to HTTP requests, notably the
//! `Request` type itself as well as a builder to create requests. Typically
//! you'll import the `http::Request` type rather than reaching into this
//! module itself.
//!
//! # Examples
//!
//! Creating a `Request` to send
//!
//! ```
//! use http::{HeaderValue, Request, Response, header};
//!
//! # fn main() -> http::Result<()> {
//! let mut request = Request::builder2()
//!     .try_uri("https://www.rust-lang.org/")?
//!     .header(header::USER_AGENT, HeaderValue::from_static("my-awesome-agent/1.0"));
//!
//! if needs_awesome_header() {
//!     request = request.try_header("Awesome", "yes")?;
//! }
//!
//! let response = send(request.body(()));
//! # Ok(())
//! # }
//! # fn needs_awesome_header() -> bool {
//! #     true
//! # }
//!
//! fn send(req: Request<()>) -> Response<()> {
//!     // ...
//! #   Response::builder().body(()).unwrap()
//! }
//! ```
//!
//! Inspecting a request to see what was sent.
//!
//! ```
//! use http::{Request, Response, StatusCode};
//!
//! fn respond_to(req: Request<()>) -> http::Result<Response<()>> {
//!     if req.uri() != "/awesome-url" {
//!         return Response::builder()
//!             .status(StatusCode::NOT_FOUND)
//!             .body(())
//!     }
//!
//!     let has_awesome_header = req.headers().contains_key("Awesome");
//!     let body = req.body();
//!
//!     // ...
//! # panic!()
//! }
//! ```

use std::any::Any;
use std::convert::{TryFrom};
use std::fmt;

use crate::header::{HeaderMap, HeaderName, HeaderValue};
use crate::method::Method;
use crate::version::Version;
use crate::{Extensions, Result, Uri};

/// Represents an HTTP request.
///
/// An HTTP request consists of a head and a potentially optional body. The body
/// component is generic, enabling arbitrary types to represent the HTTP body.
/// For example, the body could be `Vec<u8>`, a `Stream` of byte chunks, or a
/// value that has been deserialized.
///
/// # Examples
///
/// Creating a `Request` to send
///
/// ```
/// use http::{HeaderValue, Request, Response, header};
///
/// # fn main() -> http::Result<()> {
/// let mut request = Request::builder2()
///     .try_uri("https://www.rust-lang.org/")?
///     .header(header::USER_AGENT, HeaderValue::from_static("my-awesome-agent/1.0"));
///
/// if needs_awesome_header() {
///     request = request.try_header("Awesome", "yes")?;
/// }
///
/// let response = send(request.body(()));
/// # Ok(())
/// # }
/// # fn needs_awesome_header() -> bool {
/// #     true
/// # }
///
/// fn send(req: Request<()>) -> Response<()> {
///     // ...
/// #   Response::builder().body(()).unwrap()
/// }
/// ```
///
/// Inspecting a request to see what was sent.
///
/// ```
/// use http::{Request, Response, StatusCode};
///
/// fn respond_to(req: Request<()>) -> http::Result<Response<()>> {
///     if req.uri() != "/awesome-url" {
///         return Response::builder()
///             .status(StatusCode::NOT_FOUND)
///             .body(())
///     }
///
///     let has_awesome_header = req.headers().contains_key("Awesome");
///     let body = req.body();
///
///     // ...
/// # panic!()
/// }
/// ```
///
/// Deserialize a request of bytes via json:
///
/// ```
/// # extern crate serde;
/// # extern crate serde_json;
/// # extern crate http;
/// use http::Request;
/// use serde::de;
///
/// fn deserialize<T>(req: Request<Vec<u8>>) -> serde_json::Result<Request<T>>
///     where for<'de> T: de::Deserialize<'de>,
/// {
///     let (parts, body) = req.into_parts();
///     let body = serde_json::from_slice(&body)?;
///     Ok(Request::from_parts(parts, body))
/// }
/// #
/// # fn main() {}
/// ```
///
/// Or alternatively, serialize the body of a request to json
///
/// ```
/// # extern crate serde;
/// # extern crate serde_json;
/// # extern crate http;
/// use http::Request;
/// use serde::ser;
///
/// fn serialize<T>(req: Request<T>) -> serde_json::Result<Request<Vec<u8>>>
///     where T: ser::Serialize,
/// {
///     let (parts, body) = req.into_parts();
///     let body = serde_json::to_vec(&body)?;
///     Ok(Request::from_parts(parts, body))
/// }
/// #
/// # fn main() {}
/// ```
pub struct Request<T> {
    head: Parts,
    body: T,
}

/// Component parts of an HTTP `Request`
///
/// The HTTP request head consists of a method, uri, version, and a set of
/// header fields.
pub struct Parts {
    /// The request's method
    pub method: Method,

    /// The request's URI
    pub uri: Uri,

    /// The request's version
    pub version: Version,

    /// The request's headers
    pub headers: HeaderMap<HeaderValue>,

    /// The request's extensions
    pub extensions: Extensions,

    _priv: (),
}

/// An HTTP request builder
///
/// This type can be used to construct an instance or `Request`
/// through a builder-like pattern.
#[derive(Debug)]
pub struct Builder {
    inner: Result<Parts>,
}

impl Request<()> {
    /// Creates a new builder-style object to manufacture a `Request`
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::builder()
    ///     .method("GET")
    ///     .uri("https://www.rust-lang.org/")
    ///     .header("X-Custom-Foo", "Bar")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[inline]
    #[deprecated(note="Please use builder2")]
    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Creates a new `Builder` initialized with a GET method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::get("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note="Please use get2")]
    pub fn get<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,

    {
        Builder::new().method(Method::GET).uri(uri)
    }

    /// Creates a new `Builder` initialized with a PUT method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::put("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note="Please use put2")]
    pub fn put<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,

    {
        Builder::new().method(Method::PUT).uri(uri)
    }

    /// Creates a new `Builder` initialized with a POST method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::post("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note="Please use post2")]
    pub fn post<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,

    {
        Builder::new().method(Method::POST).uri(uri)
    }

    /// Creates a new `Builder` initialized with a DELETE method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::delete("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note="Please use delete2")]
    pub fn delete<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,

    {
        Builder::new().method(Method::DELETE).uri(uri)
    }

    /// Creates a new `Builder` initialized with an OPTIONS method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::options("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// # assert_eq!(*request.method(), Method::OPTIONS);
    /// ```
    #[deprecated(note="Please use options2")]
    pub fn options<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,

    {
        Builder::new().method(Method::OPTIONS).uri(uri)
    }

    /// Creates a new `Builder` initialized with a HEAD method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::head("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note="Please use head2")]
    pub fn head<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,

    {
        Builder::new().method(Method::HEAD).uri(uri)
    }

    /// Creates a new `Builder` initialized with a CONNECT method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::connect("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note="Please use connect2")]
    pub fn connect<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,

    {
        Builder::new().method(Method::CONNECT).uri(uri)
    }

    /// Creates a new `Builder` initialized with a PATCH method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::patch("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note="Please use patch2")]
    pub fn patch<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        Builder::new().method(Method::PATCH).uri(uri)
    }

    /// Creates a new `Builder` initialized with a TRACE method and the given URI.
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::trace("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note="Please use trace2")]
    pub fn trace<T>(uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        Builder::new().method(Method::TRACE).uri(uri)
    }
}

/// An HTTP request builder
///
/// This type can be used to construct an instance or `Request`
/// through a builder-like pattern.
#[derive(Debug)]
pub struct Builder2 {
    inner: Parts,
}

impl Request<()> {
    /// Creates a new builder-style object to manufacture a `Request`
    ///
    /// This method returns an instance of `Builder2` which can be used to
    /// create a `Request`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # fn main() -> Result<()> {
    /// let request = Request::builder2()
    ///     .method(Method::GET)
    ///     .uri(Uri::from_static("https://www.rust-lang.org/"))
    ///     .try_header("X-Custom-Foo", "Bar")?
    ///     .body(());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn builder2() -> Builder2 {
        Builder2::new()
    }

    /// Creates a new builder initialized with a GET method and the given URI.
    ///
    /// This method returns an instance of `Builder2` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::get2(Uri::from_static("https://www.rust-lang.org/"))
    ///     .body(());
    /// ```
    pub fn get2<T>(uri: T) -> Builder2
    where
        Uri: From<T>,
    {
        Builder2::new().method(Method::GET).uri(uri)
    }

    /// Creates a new builder initialized with a PUT method and the given URI.
    ///
    /// This method returns an instance of `Builder2` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::put2(Uri::from_static("https://www.rust-lang.org/"))
    ///     .body(());
    /// ```
    pub fn put2<T>(uri: T) -> Builder2
    where
        Uri: From<T>,
    {
        Builder2::new().method(Method::PUT).uri(uri)
    }

    /// Creates a new builder initialized with a POST method and the given URI.
    ///
    /// This method returns an instance of `Builder2` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::post2(Uri::from_static("https://www.rust-lang.org/"))
    ///     .body(());
    /// ```
    pub fn post2<T>(uri: T) -> Builder2
    where
        Uri: From<T>,
    {
        Builder2::new().method(Method::POST).uri(uri)
    }

    /// Creates a new builder initialized with a DELETE method and the given URI.
    ///
    /// This method returns an instance of `Builder2` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::delete2(Uri::from_static("https://www.rust-lang.org/"))
    ///     .body(());
    /// ```
    pub fn delete2<T>(uri: T) -> Builder2
    where
        Uri: From<T>,
    {
        Builder2::new().method(Method::DELETE).uri(uri)
    }

    /// Creates a new builder initialized with an OPTIONS method and the given URI.
    ///
    /// This method returns an instance of `Builder2` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::options2(Uri::from_static("https://www.rust-lang.org/"))
    ///     .body(());
    /// assert_eq!(request.method(), Method::OPTIONS);
    /// ```
    pub fn options2<T>(uri: T) -> Builder2
    where
        Uri: From<T>,
    {
        Builder2::new().method(Method::OPTIONS).uri(uri)
    }

    /// Creates a new builder initialized with a HEAD method and the given URI.
    ///
    /// This method returns an instance of `Builder2` which can be used
    /// to create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::head2(Uri::from_static("https://www.rust-lang.org/"))
    ///     .body(());
    /// ```
    pub fn head2<T>(uri: T) -> Builder2
    where
        Uri: From<T>,
    {
        Builder2::new().method(Method::HEAD).uri(uri)
    }

    /// Creates a new builder initialized with a CONNECT method and the given URI.
    ///
    /// This method returns an instance of `Builder2` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::connect2(Uri::from_static("https://www.rust-lang.org/"))
    ///     .body(());
    /// ```
    pub fn connect2<T>(uri: T) -> Builder2
    where
        Uri: From<T>,
    {
        Builder2::new().method(Method::CONNECT).uri(uri)
    }

    /// Creates a new builder initialized with a PATCH method and the given URI.
    ///
    /// This method returns an instance of `Builder2` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::patch2(Uri::from_static("https://www.rust-lang.org/"))
    ///     .body(());
    /// ```
    pub fn patch2<T>(uri: T) -> Builder2
    where
        Uri: From<T>,
    {
        Builder2::new().method(Method::PATCH).uri(uri)
    }

    /// Creates a new builder initialized with a TRACE method and the given URI.
    ///
    /// This method returns an instance of `Builder2` which can be used to
    /// create a `Request`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::trace2(Uri::from_static("https://www.rust-lang.org/"))
    ///     .body(());
    /// ```
    pub fn trace2<T>(uri: T) -> Builder2
    where
        Uri: From<T>,
    {
        Builder2::new().method(Method::TRACE).uri(uri)
    }
}

impl<T> Request<T> {
    /// Creates a new blank `Request` with the body
    ///
    /// The component parts of this request will be set to their default, e.g.
    /// the GET method, no headers, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::new("hello world");
    ///
    /// assert_eq!(*request.method(), Method::GET);
    /// assert_eq!(*request.body(), "hello world");
    /// ```
    #[inline]
    pub fn new(body: T) -> Request<T> {
        Request {
            head: Parts::new(),
            body: body,
        }
    }

    /// Creates a new `Request` with the given components parts and body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::new("hello world");
    /// let (mut parts, body) = request.into_parts();
    /// parts.method = Method::POST;
    ///
    /// let request = Request::from_parts(parts, body);
    /// ```
    #[inline]
    pub fn from_parts(parts: Parts, body: T) -> Request<T> {
        Request {
            head: parts,
            body: body,
        }
    }

    /// Returns a reference to the associated HTTP method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request: Request<()> = Request::default();
    /// assert_eq!(*request.method(), Method::GET);
    /// ```
    #[inline]
    pub fn method(&self) -> &Method {
        &self.head.method
    }

    /// Returns a mutable reference to the associated HTTP method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut request: Request<()> = Request::default();
    /// *request.method_mut() = Method::PUT;
    /// assert_eq!(*request.method(), Method::PUT);
    /// ```
    #[inline]
    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.head.method
    }

    /// Returns a reference to the associated URI.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request: Request<()> = Request::default();
    /// assert_eq!(*request.uri(), *"/");
    /// ```
    #[inline]
    pub fn uri(&self) -> &Uri {
        &self.head.uri
    }

    /// Returns a mutable reference to the associated URI.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut request: Request<()> = Request::default();
    /// *request.uri_mut() = "/hello".parse().unwrap();
    /// assert_eq!(*request.uri(), *"/hello");
    /// ```
    #[inline]
    pub fn uri_mut(&mut self) -> &mut Uri {
        &mut self.head.uri
    }

    /// Returns the associated version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request: Request<()> = Request::default();
    /// assert_eq!(request.version(), Version::HTTP_11);
    /// ```
    #[inline]
    pub fn version(&self) -> Version {
        self.head.version
    }

    /// Returns a mutable reference to the associated version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut request: Request<()> = Request::default();
    /// *request.version_mut() = Version::HTTP_2;
    /// assert_eq!(request.version(), Version::HTTP_2);
    /// ```
    #[inline]
    pub fn version_mut(&mut self) -> &mut Version {
        &mut self.head.version
    }

    /// Returns a reference to the associated header field map.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request: Request<()> = Request::default();
    /// assert!(request.headers().is_empty());
    /// ```
    #[inline]
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
    /// let mut request: Request<()> = Request::default();
    /// request.headers_mut().insert(HOST, HeaderValue::from_static("world"));
    /// assert!(!request.headers().is_empty());
    /// ```
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        &mut self.head.headers
    }

    /// Returns a reference to the associated extensions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request: Request<()> = Request::default();
    /// assert!(request.extensions().get::<i32>().is_none());
    /// ```
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.head.extensions
    }

    /// Returns a mutable reference to the associated extensions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::*;
    /// let mut request: Request<()> = Request::default();
    /// request.extensions_mut().insert("hello");
    /// assert_eq!(request.extensions().get(), Some(&"hello"));
    /// ```
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.head.extensions
    }

    /// Returns a reference to the associated HTTP body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request: Request<String> = Request::default();
    /// assert!(request.body().is_empty());
    /// ```
    #[inline]
    pub fn body(&self) -> &T {
        &self.body
    }

    /// Returns a mutable reference to the associated HTTP body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut request: Request<String> = Request::default();
    /// request.body_mut().push_str("hello world");
    /// assert!(!request.body().is_empty());
    /// ```
    #[inline]
    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }

    /// Consumes the request, returning just the body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::Request;
    /// let request = Request::new(10);
    /// let body = request.into_body();
    /// assert_eq!(body, 10);
    /// ```
    #[inline]
    pub fn into_body(self) -> T {
        self.body
    }

    /// Consumes the request returning the head and body parts.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::new(());
    /// let (parts, body) = request.into_parts();
    /// assert_eq!(parts.method, Method::GET);
    /// ```
    #[inline]
    pub fn into_parts(self) -> (Parts, T) {
        (self.head, self.body)
    }

    /// Consumes the request returning a new request with body mapped to the
    /// return type of the passed in function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::builder().body("some string").unwrap();
    /// let mapped_request: Request<&[u8]> = request.map(|b| {
    ///   assert_eq!(b, "some string");
    ///   b.as_bytes()
    /// });
    /// assert_eq!(mapped_request.body(), &"some string".as_bytes());
    /// ```
    #[inline]
    pub fn map<F, U>(self, f: F) -> Request<U>
    where
        F: FnOnce(T) -> U,
    {
        Request {
            body: f(self.body),
            head: self.head,
        }
    }
}

impl<T: Default> Default for Request<T> {
    fn default() -> Request<T> {
        Request::new(T::default())
    }
}

impl<T: fmt::Debug> fmt::Debug for Request<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", self.method())
            .field("uri", self.uri())
            .field("version", &self.version())
            .field("headers", self.headers())
            // omits Extensions because not useful
            .field("body", self.body())
            .finish()
    }
}

impl Parts {
    /// Creates a new default instance of `Parts`
    fn new() -> Parts {
        Parts {
            method: Method::default(),
            uri: Uri::default(),
            version: Version::default(),
            headers: HeaderMap::default(),
            extensions: Extensions::default(),
            _priv: (),
        }
    }
}

impl fmt::Debug for Parts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Parts")
            .field("method", &self.method)
            .field("uri", &self.uri)
            .field("version", &self.version)
            .field("headers", &self.headers)
            // omits Extensions because not useful
            // omits _priv because not useful
            .finish()
    }
}

impl Builder {
    /// Creates a new default instance of `Builder` to construct a `Request`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let req = request::Builder::new()
    ///     .method("POST")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[inline]
    pub fn new() -> Builder {
        Builder::default()
    }

    /// Set the HTTP method for this request.
    ///
    /// This function will configure the HTTP method of the `Request` that will
    /// be returned from `Builder::build`.
    ///
    /// By default this is `GET`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let req = Request::builder()
    ///     .method("POST")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn method<T>(self, method: T) -> Builder
    where
        Method: TryFrom<T>,
        <Method as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.and_then(move |mut head| {
            let method = TryFrom::try_from(method).map_err(Into::into)?;
            head.method = method;
            Ok(head)
        })
    }

    /// Get the HTTP Method for this request.
    ///
    /// By default this is `GET`. If builder has error, returns None.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let mut req = Request::builder();
    /// assert_eq!(req.method_ref(),Some(&Method::GET));
    ///
    /// req = req.method("POST");
    /// assert_eq!(req.method_ref(),Some(&Method::POST));
    /// ```
    pub fn method_ref(&self) -> Option<&Method> {
        self.inner.as_ref().ok().map(|h| &h.method)
    }

    /// Set the URI for this request.
    ///
    /// This function will configure the URI of the `Request` that will
    /// be returned from `Builder::build`.
    ///
    /// By default this is `/`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let req = Request::builder()
    ///     .uri("https://www.rust-lang.org/")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn uri<T>(self, uri: T) -> Builder
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.and_then(move |mut head| {
            head.uri = TryFrom::try_from(uri).map_err(Into::into)?;
            Ok(head)
        })
    }

    /// Get the URI for this request
    ///
    /// By default this is `/`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let mut req = Request::builder();
    /// assert_eq!(req.uri_ref().unwrap(), "/" );
    ///
    /// req = req.uri("https://www.rust-lang.org/");
    /// assert_eq!(req.uri_ref().unwrap(), "https://www.rust-lang.org/" );
    /// ```
    pub fn uri_ref(&self) -> Option<&Uri> {
        self.inner.as_ref().ok().map(|h| &h.uri)
    }

    /// Set the HTTP version for this request.
    ///
    /// This function will configure the HTTP version of the `Request` that
    /// will be returned from `Builder::build`.
    ///
    /// By default this is HTTP/1.1
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let req = Request::builder()
    ///     .version(Version::HTTP_2)
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn version(self, version: Version) -> Builder {
        self.and_then(move |mut head| {
            head.version = version;
            Ok(head)
        })
    }

    /// Appends a header to this request builder.
    ///
    /// This function will append the provided key/value as a header to the
    /// internal `HeaderMap` being constructed. Essentially this is equivalent
    /// to calling `HeaderMap::append`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::HeaderValue;
    ///
    /// let req = Request::builder()
    ///     .header("Accept", "text/html")
    ///     .header("X-Custom-Foo", "bar")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn header<K, V>(self, key: K, value: V) -> Builder
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<crate::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<crate::Error>,
    {
        self.and_then(move |mut head| {
            let name = <HeaderName as TryFrom<K>>::try_from(key).map_err(Into::into)?;
            let value = <HeaderValue as TryFrom<V>>::try_from(value).map_err(Into::into)?;
            head.headers.append(name, value);
            Ok(head)
        })
    }

    /// Get header on this request builder.
    /// when builder has error returns None
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Request;
    /// let req = Request::builder()
    ///     .header("Accept", "text/html")
    ///     .header("X-Custom-Foo", "bar");
    /// let headers = req.headers_ref().unwrap();
    /// assert_eq!( headers["Accept"], "text/html" );
    /// assert_eq!( headers["X-Custom-Foo"], "bar" );
    /// ```
    pub fn headers_ref(&self) -> Option<&HeaderMap<HeaderValue>> {
        self.inner.as_ref().ok().map(|h| &h.headers)
    }

    /// Get headers on this request builder.
    ///
    /// When builder has error returns None.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::{header::HeaderValue, Request};
    /// let mut req = Request::builder();
    /// {
    ///   let headers = req.headers_mut().unwrap();
    ///   headers.insert("Accept", HeaderValue::from_static("text/html"));
    ///   headers.insert("X-Custom-Foo", HeaderValue::from_static("bar"));
    /// }
    /// let headers = req.headers_ref().unwrap();
    /// assert_eq!( headers["Accept"], "text/html" );
    /// assert_eq!( headers["X-Custom-Foo"], "bar" );
    /// ```
    pub fn headers_mut(&mut self) -> Option<&mut HeaderMap<HeaderValue>> {
        self.inner.as_mut().ok().map(|h| &mut h.headers)
    }

    /// Adds an extension to this builder
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let req = Request::builder()
    ///     .extension("My Extension")
    ///     .body(())
    ///     .unwrap();
    ///
    /// assert_eq!(req.extensions().get::<&'static str>(),
    ///            Some(&"My Extension"));
    /// ```
    pub fn extension<T>(self, extension: T) -> Builder
    where
        T: Any + Send + Sync + 'static,
    {
        self.and_then(move |mut head| {
            head.extensions.insert(extension);
            Ok(head)
        })
    }

    /// Get a reference to the extensions for this request builder.
    ///
    /// If the builder has an error, this returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Request;
    /// let req = Request::builder().extension("My Extension").extension(5u32);
    /// let extensions = req.extensions_ref().unwrap();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    pub fn extensions_ref(&self) -> Option<&Extensions> {
        self.inner.as_ref().ok().map(|h| &h.extensions)
    }

    /// Get a mutable reference to the extensions for this request builder.
    ///
    /// If the builder has an error, this returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Request;
    /// let mut req = Request::builder().extension("My Extension");
    /// let mut extensions = req.extensions_mut().unwrap();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// extensions.insert(5u32);
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    pub fn extensions_mut(&mut self) -> Option<&mut Extensions> {
        self.inner.as_mut().ok().map(|h| &mut h.extensions)
    }

    /// "Consumes" this builder, using the provided `body` to return a
    /// constructed `Request`.
    ///
    /// # Errors
    ///
    /// This function may return an error if any previously configured argument
    /// failed to parse or get converted to the internal representation. For
    /// example if an invalid `head` was specified via `header("Foo",
    /// "Bar\r\n")` the error will be returned when this function is called
    /// rather than when `header` was called.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::builder()
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn body<T>(self, body: T) -> Result<Request<T>> {
        self.inner.map(move |head| {
            Request {
                head,
                body,
            }
        })
    }

    // private

    fn and_then<F>(self, func: F) -> Self
    where
        F: FnOnce(Parts) -> Result<Parts>
    {
        Builder {
            inner: self.inner.and_then(func),
        }
    }
}

impl Default for Builder {
    #[inline]
    fn default() -> Builder {
        Builder {
            inner: Ok(Parts::new()),
        }
    }
}

impl Builder2 {
    /// Creates a new default instance of `Builder2` to construct a `Request`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let req = request::Builder2::new()
    ///     .method(Method::POST)
    ///     .body(());
    /// ```
    #[inline]
    pub fn new() -> Builder2 {
        Builder2::default()
    }

    /// Set the HTTP method for this request.
    ///
    /// This function will configure the HTTP method of the `Request` that will
    /// be returned from `Builder2::build`.
    ///
    /// By default this is `GET`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let req = Request::builder2()
    ///     .method(Method::POST)
    ///     .body(());
    /// ```
    pub fn method<T>(mut self, method: T) -> Builder2
    where
        Method: From<T>,
    {
        self.inner.method = method.into();
        self
    }

    /// Get the HTTP Method for this request.
    ///
    /// By default this is `GET`. If builder has error, returns None.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let mut req = Request::builder2();
    /// assert_eq!(req.method_ref(), Method::GET);
    ///
    /// req = req.method(Method::POST);
    /// assert_eq!(req.method_ref(), Method::POST);
    /// ```
    pub fn method_ref(&self) -> &Method {
        &self.inner.method
    }

    /// Set the URI for this request.
    ///
    /// This function will configure the URI of the `Request` that will
    /// be returned from `Builder2::build`.
    ///
    /// By default this is `/`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let req = Request::builder2()
    ///     .uri(Uri::from_static("https://www.rust-lang.org/"))
    ///     .body(());
    /// ```
    pub fn uri<T>(mut self, uri: T) -> Builder2
    where
        Uri: From<T>,
    {
        self.inner.uri = uri.into();
        self
    }

    /// Set the URI for this request.
    ///
    /// This function will configure the URI of the `Request` that will
    /// be returned from `Builder2::build`.
    ///
    /// By default this is `/`.
    ///
    /// # Errors
    ///
    /// This method does a fallible conversion, and returns an error if
    /// the conversion fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # fn main() -> Result<()> {
    /// let req = Request::builder2()
    ///     .try_uri("https://www.rust-lang.org/")?
    ///     .body(());
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_uri<T>(mut self, uri: T) -> Result<Builder2>
    where
        Uri: TryFrom<T>,
        crate::Error: From<<Uri as TryFrom<T>>::Error>,
    {
        self.inner.uri = Uri::try_from(uri)?;
        Ok(self)
    }

    /// Get the URI for this request
    ///
    /// By default this is `/`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # fn main() -> Result<()> {
    /// let mut req = Request::builder2();
    /// assert_eq!(req.uri_ref(), "/" );
    ///
    /// req = req.try_uri("https://www.rust-lang.org/")?;
    /// assert_eq!(req.uri_ref(), "https://www.rust-lang.org/");
    /// # Ok(())
    /// # }
    /// ```
    pub fn uri_ref(&self) -> &Uri {
        &self.inner.uri
    }

    /// Set the HTTP version for this request.
    ///
    /// This function will configure the HTTP version of the `Request` that
    /// will be returned from `Builder2::build`.
    ///
    /// By default this is HTTP/1.1
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let req = Request::builder2()
    ///     .version(Version::HTTP_2)
    ///     .body(());
    /// ```
    pub fn version(mut self, version: Version) -> Builder2 {
        self.inner.version = version;
        self
    }

    /// Appends a header to this request builder.
    ///
    /// This function will append the provided key/value as a header to the
    /// internal `HeaderMap` being constructed. Essentially this is equivalent
    /// to calling `HeaderMap::append`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::{HeaderName, HeaderValue};
    /// # #[allow(non_snake_case)] // look like mime::TEXT_HTML
    /// # let TEXT_HTML: HeaderValue = HeaderValue::from_static("text/html");
    ///
    /// let req = Request::builder2()
    ///     .header(header::ACCEPT, TEXT_HTML)
    ///     .header(
    ///         HeaderName::from_static("x-custom-foo"),
    ///         HeaderValue::from_static("bar"),
    ///     )
    ///     .body(());
    /// ```
    pub fn header<K, V>(mut self, key: K, value: V) -> Builder2
    where
        HeaderName: From<K>,
        HeaderValue: From<V>,
    {
        self.inner.headers.append(HeaderName::from(key), value.into());
        self
    }

    /// Appends a header to this request builder.
    ///
    /// This function will append the provided key/value as a header to the
    /// internal `HeaderMap` being constructed. Essentially this is equivalent
    /// to calling `HeaderMap::append`.
    ///
    /// # Errors
    ///
    /// This method does fallible conversions, and returns an error if
    /// one of the conversions fail.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # fn main() -> Result<()> {
    /// let req = Request::builder2()
    ///     .try_header("Accept", "text/html")?
    ///     .try_header("X-Custom-Foo", "bar")?
    ///     .body(());
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_header<K, V>(mut self, key: K, value: V) -> Result<Builder2>
    where
        HeaderName: TryFrom<K>,
        crate::Error: From<<HeaderName as TryFrom<K>>::Error>,
        HeaderValue: TryFrom<V>,
        crate::Error: From<<HeaderValue as TryFrom<V>>::Error>,
    {
        let name = HeaderName::try_from(key)?;
        let value = HeaderValue::try_from(value)?;
        self.inner.headers.append(name, value);
        Ok(self)
    }

    /// Get header on this request builder.
    /// when builder has error returns None
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Request;
    /// # fn main() -> Result<(), http::Error> {
    /// let req = Request::builder2()
    ///     .try_header("Accept", "text/html")?
    ///     .try_header("X-Custom-Foo", "bar")?;
    /// let headers = req.headers_ref();
    /// assert_eq!(headers["Accept"], "text/html");
    /// assert_eq!(headers["X-Custom-Foo"], "bar");
    /// # Ok(())
    /// # }
    /// ```
    pub fn headers_ref(&self) -> &HeaderMap<HeaderValue> {
        &self.inner.headers
    }

    /// Get headers on this request builder.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::{header::HeaderValue, Request};
    /// let mut req = Request::builder2();
    /// {
    ///   let headers = req.headers_mut();
    ///   headers.insert("Accept", HeaderValue::from_static("text/html"));
    ///   headers.insert("X-Custom-Foo", HeaderValue::from_static("bar"));
    /// }
    /// let headers = req.headers_ref();
    /// assert_eq!( headers["Accept"], "text/html" );
    /// assert_eq!( headers["X-Custom-Foo"], "bar" );
    /// ```
    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        &mut self.inner.headers
    }

    /// Adds an extension to this builder
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let req = Request::builder2()
    ///     .extension("My Extension")
    ///     .body(());
    ///
    /// assert_eq!(req.extensions().get::<&'static str>(),
    ///            Some(&"My Extension"));
    /// ```
    pub fn extension<T>(mut self, extension: T) -> Builder2
    where
        T: Any + Send + Sync + 'static,
    {
        self.inner.extensions.insert(extension);
        self
    }

    /// Get a reference to the extensions for this request builder.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Request;
    /// let req = Request::builder2()
    ///     .extension("My Extension")
    ///     .extension(5u32);
    /// let extensions = req.extensions_ref();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    pub fn extensions_ref(&self) -> &Extensions {
        &self.inner.extensions
    }

    /// Get a mutable reference to the extensions for this request builder.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Request;
    /// let mut req = Request::builder2().extension("My Extension");
    /// let mut extensions = req.extensions_mut();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// extensions.insert(5u32);
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.inner.extensions
    }

    /// "Consumes" this builder, using the provided `body` to return a
    /// constructed `Request`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = Request::builder2()
    ///     .body(());
    /// ```
    pub fn body<T>(self, body: T) -> Request<T> {
        Request {
            head: self.inner,
            body,
        }
    }
}

impl Default for Builder2 {
    #[inline]
    fn default() -> Builder2 {
        Builder2 {
            inner: Parts::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_map_a_body_from_one_type_to_another() {
        let request = Request::builder2().body("some string");
        let mapped_request = request.map(|s| {
            assert_eq!(s, "some string");
            123u32
        });
        assert_eq!(mapped_request.body(), &123u32);
    }
}
