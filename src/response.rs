//! HTTP response types.
//!
//! This module contains structs related to HTTP responses, notably the
//! `Response` type itself as well as a builder to create responses. Typically
//! you'll import the `http::Response` type rather than reaching into this
//! module itself.
//!
//! # Examples
//!
//! Creating a `Response` to return
//!
//! ```
//! use http::{Request, Response, StatusCode};
//!
//! fn respond_to(req: Request<()>) -> http::Result<Response<()>> {
//!     let mut builder = Response::builder()
//!         .header("Foo", "Bar")
//!         .status(StatusCode::OK);
//!
//!     if req.headers().contains_key("Another-Header") {
//!         builder = builder.header("Another-Header", "Ack");
//!     }
//!
//!     builder.body(())
//! }
//! ```
//!
//! A simple 404 handler
//!
//! ```
//! use http::{Request, Response, StatusCode};
//!
//! fn not_found(_req: Request<()>) -> http::Result<Response<()>> {
//!     Response::builder()
//!         .status(StatusCode::NOT_FOUND)
//!         .body(())
//! }
//! ```
//!
//! Or otherwise inspecting the result of a request:
//!
//! ```no_run
//! use http::{Request, Response};
//!
//! fn get(url: &str) -> http::Result<Response<()>> {
//!     // ...
//! # panic!()
//! }
//!
//! let response = get("https://www.rust-lang.org/").unwrap();
//!
//! if !response.status().is_success() {
//!     panic!("failed to get a successful response status!");
//! }
//!
//! if let Some(date) = response.headers().get("Date") {
//!     // we've got a `Date` header!
//! }
//!
//! let body = response.body();
//! // ...
//! ```

use std::any::Any;
use std::convert::TryFrom;
use std::fmt;

use crate::header::{HeaderMap, HeaderName, HeaderValue};
use crate::status::StatusCode;
use crate::version::Version;
use crate::{Extensions, Result};

/// Represents an HTTP response
///
/// An HTTP response consists of a head and a potentially optional body. The body
/// component is generic, enabling arbitrary types to represent the HTTP body.
/// For example, the body could be `Vec<u8>`, a `Stream` of byte chunks, or a
/// value that has been deserialized.
///
/// Typically you'll work with responses on the client side as the result of
/// sending a `Request` and on the server you'll be generating a `Response` to
/// send back to the client.
///
/// # Examples
///
/// Creating a `Response` to return
///
/// ```
/// use http::{Request, Response, StatusCode};
///
/// fn respond_to(req: Request<()>) -> http::Result<Response<()>> {
///     let mut builder = Response::builder()
///         .header("Foo", "Bar")
///         .status(StatusCode::OK);
///
///     if req.headers().contains_key("Another-Header") {
///         builder = builder.header("Another-Header", "Ack");
///     }
///
///     builder.body(())
/// }
/// ```
///
/// A simple 404 handler
///
/// ```
/// use http::{Request, Response, StatusCode};
///
/// fn not_found(_req: Request<()>) -> http::Result<Response<()>> {
///     Response::builder()
///         .status(StatusCode::NOT_FOUND)
///         .body(())
/// }
/// ```
///
/// Or otherwise inspecting the result of a request:
///
/// ```no_run
/// use http::{Request, Response};
///
/// fn get(url: &str) -> http::Result<Response<()>> {
///     // ...
/// # panic!()
/// }
///
/// let response = get("https://www.rust-lang.org/").unwrap();
///
/// if !response.status().is_success() {
///     panic!("failed to get a successful response status!");
/// }
///
/// if let Some(date) = response.headers().get("Date") {
///     // we've got a `Date` header!
/// }
///
/// let body = response.body();
/// // ...
/// ```
///
/// Deserialize a response of bytes via json:
///
/// ```
/// # extern crate serde;
/// # extern crate serde_json;
/// # extern crate http;
/// use http::Response;
/// use serde::de;
///
/// fn deserialize<T>(res: Response<Vec<u8>>) -> serde_json::Result<Response<T>>
///     where for<'de> T: de::Deserialize<'de>,
/// {
///     let (parts, body) = res.into_parts();
///     let body = serde_json::from_slice(&body)?;
///     Ok(Response::from_parts(parts, body))
/// }
/// #
/// # fn main() {}
/// ```
///
/// Or alternatively, serialize the body of a response to json
///
/// ```
/// # extern crate serde;
/// # extern crate serde_json;
/// # extern crate http;
/// use http::Response;
/// use serde::ser;
///
/// fn serialize<T>(res: Response<T>) -> serde_json::Result<Response<Vec<u8>>>
///     where T: ser::Serialize,
/// {
///     let (parts, body) = res.into_parts();
///     let body = serde_json::to_vec(&body)?;
///     Ok(Response::from_parts(parts, body))
/// }
/// #
/// # fn main() {}
/// ```
pub struct Response<T> {
    head: Parts,
    body: T,
}

/// Component parts of an HTTP `Response`
///
/// The HTTP response head consists of a status, version, and a set of
/// header fields.
pub struct Parts {
    /// The response's status
    pub status: StatusCode,

    /// The response's version
    pub version: Version,

    /// The response's headers
    pub headers: HeaderMap<HeaderValue>,

    /// The response's extensions
    pub extensions: Extensions,

    _priv: (),
}

/// An HTTP response builder
///
/// This type can be used to construct an instance of `Response` through a
/// builder-like pattern.
/// This builder can represent an erroneous state, so finalizing
/// it (with the `.body` method) may return an error.
///
/// See also [`Builder2`].
#[derive(Debug)]
// Note: rustc 1.39.0 does some use I can't find, so I can't deprecate
// the type.  Instead, I have just deprecated every function that
// returns or consumes a Builder.
//#[deprecated(note = "Please use Builder2, it will replace Builder")]
pub struct Builder {
    inner: Result<Parts>,
}

/// An HTTP response builder
///
/// This type can be used to construct an instance of `Response` through a
/// builder-like pattern.
///
/// This builder can not represent an erroneous state, so as long
/// as you have a `Builder2` you can get a `Response`.
/// Most methods on this builder is guaranteed to return a builder.
/// The exception, `try_header()`, is explicit by returning a
/// `Result<Builder2>`.
///
/// See also [`Builder`]
#[derive(Debug)]
pub struct Builder2 {
    inner: Parts,
}

impl Response<()> {
    /// Creates a new builder-style object to manufacture a `Response`
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create a `Result<Response>`.
    /// This builder can represent an erroneous state, so finalizing
    /// it (with the `.body` method) may return an error.
    ///
    /// See also [`builder2`](#method.builder2).
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response = Response::builder()
    ///     .status(200)
    ///     .header("X-Custom-Foo", "Bar")
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[inline]
    #[deprecated(note = "Please use Self::builder2, it will replace Builder")]
    pub fn builder() -> Builder {
        #[allow(deprecated)]
        Builder::new()
    }

    /// Creates a new builder-style object to manufacture a `Response`
    ///
    /// This method returns an instance of `Builder2` which can be used to
    /// create a `Response`.
    /// This builder can not represent an erroneous state, so as long
    /// as you have a `Builder2` you can get a `Response`.
    ///
    /// See also [`builder`](#method.builder).
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::CONTENT_TYPE;
    /// let response = Response::builder2()
    ///     .status(StatusCode::OK)
    ///     .header(CONTENT_TYPE, HeaderValue::from_static("text/html"))
    ///     .body(());
    /// ```
    #[inline]
    pub fn builder2() -> Builder2 {
        Builder2::new()
    }
}

impl<T> Response<T> {
    /// Creates a new blank `Response` with the body
    ///
    /// The component ports of this response will be set to their default, e.g.
    /// the ok status, no headers, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response = Response::new("hello world");
    ///
    /// assert_eq!(response.status(), StatusCode::OK);
    /// assert_eq!(*response.body(), "hello world");
    /// ```
    #[inline]
    pub fn new(body: T) -> Response<T> {
        Response {
            head: Parts::new(),
            body: body,
        }
    }

    /// Creates a new `Response` with the given head and body
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response = Response::new("hello world");
    /// let (mut parts, body) = response.into_parts();
    ///
    /// parts.status = StatusCode::BAD_REQUEST;
    /// let response = Response::from_parts(parts, body);
    ///
    /// assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    /// assert_eq!(*response.body(), "hello world");
    /// ```
    #[inline]
    pub fn from_parts(parts: Parts, body: T) -> Response<T> {
        Response {
            head: parts,
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
    /// assert_eq!(response.status(), StatusCode::OK);
    /// ```
    #[inline]
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
    /// *response.status_mut() = StatusCode::CREATED;
    /// assert_eq!(response.status(), StatusCode::CREATED);
    /// ```
    #[inline]
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
    /// assert_eq!(response.version(), Version::HTTP_11);
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
    /// let mut response: Response<()> = Response::default();
    /// *response.version_mut() = Version::HTTP_2;
    /// assert_eq!(response.version(), Version::HTTP_2);
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
    /// let response: Response<()> = Response::default();
    /// assert!(response.headers().is_empty());
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
    /// let mut response: Response<()> = Response::default();
    /// response.headers_mut().insert(HOST, HeaderValue::from_static("world"));
    /// assert!(!response.headers().is_empty());
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
    /// let response: Response<()> = Response::default();
    /// assert!(response.extensions().get::<i32>().is_none());
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
    /// let mut response: Response<()> = Response::default();
    /// response.extensions_mut().insert("hello");
    /// assert_eq!(response.extensions().get(), Some(&"hello"));
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
    /// let response: Response<String> = Response::default();
    /// assert!(response.body().is_empty());
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
    /// let mut response: Response<String> = Response::default();
    /// response.body_mut().push_str("hello world");
    /// assert!(!response.body().is_empty());
    /// ```
    #[inline]
    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }

    /// Consumes the response, returning just the body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::Response;
    /// let response = Response::new(10);
    /// let body = response.into_body();
    /// assert_eq!(body, 10);
    /// ```
    #[inline]
    pub fn into_body(self) -> T {
        self.body
    }

    /// Consumes the response returning the head and body parts.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<()> = Response::default();
    /// let (parts, body) = response.into_parts();
    /// assert_eq!(parts.status, StatusCode::OK);
    /// ```
    #[inline]
    pub fn into_parts(self) -> (Parts, T) {
        (self.head, self.body)
    }

    /// Consumes the response returning a new response with body mapped to the
    /// return type of the passed in function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response = Response::builder().body("some string").unwrap();
    /// let mapped_response: Response<&[u8]> = response.map(|b| {
    ///   assert_eq!(b, "some string");
    ///   b.as_bytes()
    /// });
    /// assert_eq!(mapped_response.body(), &"some string".as_bytes());
    /// ```
    #[inline]
    pub fn map<F, U>(self, f: F) -> Response<U>
    where
        F: FnOnce(T) -> U,
    {
        Response {
            body: f(self.body),
            head: self.head,
        }
    }
}

impl<T: Default> Default for Response<T> {
    #[inline]
    fn default() -> Response<T> {
        Response::new(T::default())
    }
}

impl<T: fmt::Debug> fmt::Debug for Response<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status())
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
            status: StatusCode::default(),
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
            .field("status", &self.status)
            .field("version", &self.version)
            .field("headers", &self.headers)
            // omits Extensions because not useful
            // omits _priv because not useful
            .finish()
    }
}

impl Builder {
    /// Creates a new default instance of `Builder` to construct either a
    /// `Head` or a `Response`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let response = response::Builder::new()
    ///     .status(200)
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[inline]
    #[deprecated(note = "Please use Builder2, it will replace Builder")]
    pub fn new() -> Builder {
        Builder::default()
    }

    /// Set the HTTP status for this response.
    ///
    /// This function will configure the HTTP status code of the `Response` that
    /// will be returned from `Builder::build`.
    ///
    /// By default this is `200`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let response = Response::builder()
    ///     .status(200)
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note = "Please use Builder2, it will replace Builder")]
    pub fn status<T>(self, status: T) -> Builder
    where
        StatusCode: TryFrom<T>,
        <StatusCode as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.and_then(move |mut head| {
            head.status = TryFrom::try_from(status).map_err(Into::into)?;
            Ok(head)
        })
    }

    /// Set the HTTP version for this response.
    ///
    /// This function will configure the HTTP version of the `Response` that
    /// will be returned from `Builder::build`.
    ///
    /// By default this is HTTP/1.1
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let response = Response::builder()
    ///     .version(Version::HTTP_2)
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note = "Please use Builder2, it will replace Builder")]
    pub fn version(self, version: Version) -> Builder {
        self.and_then(move |mut head| {
            head.version = version;
            Ok(head)
        })
    }

    /// Appends a header to this response builder.
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
    /// let response = Response::builder()
    ///     .header("Content-Type", "text/html")
    ///     .header("X-Custom-Foo", "bar")
    ///     .header("content-length", 0)
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note = "Please use Builder2, it will replace Builder")]
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

    /// Get header on this response builder.
    ///
    /// When builder has error returns None.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Response;
    /// # use http::header::HeaderValue;
    /// let res = Response::builder()
    ///     .header("Accept", "text/html")
    ///     .header("X-Custom-Foo", "bar");
    /// let headers = res.headers_ref().unwrap();
    /// assert_eq!( headers["Accept"], "text/html" );
    /// assert_eq!( headers["X-Custom-Foo"], "bar" );
    /// ```
    #[deprecated(note = "Please use Builder2, it will replace Builder")]
    pub fn headers_ref(&self) -> Option<&HeaderMap<HeaderValue>> {
        self.inner.as_ref().ok().map(|h| &h.headers)
    }

    /// Get header on this response builder.
    /// when builder has error returns None
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::HeaderValue;
    /// # use http::response::Builder;
    /// let mut res = Response::builder();
    /// {
    ///   let headers = res.headers_mut().unwrap();
    ///   headers.insert("Accept", HeaderValue::from_static("text/html"));
    ///   headers.insert("X-Custom-Foo", HeaderValue::from_static("bar"));
    /// }
    /// let headers = res.headers_ref().unwrap();
    /// assert_eq!( headers["Accept"], "text/html" );
    /// assert_eq!( headers["X-Custom-Foo"], "bar" );
    /// ```
    #[deprecated(note = "Please use Builder2, it will replace Builder")]
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
    /// let response = Response::builder()
    ///     .extension("My Extension")
    ///     .body(())
    ///     .unwrap();
    ///
    /// assert_eq!(response.extensions().get::<&'static str>(),
    ///            Some(&"My Extension"));
    /// ```
    #[deprecated(note = "Please use Builder2, it will replace Builder")]
    pub fn extension<T>(self, extension: T) -> Builder
    where
        T: Any + Send + Sync + 'static,
    {
        self.and_then(move |mut head| {
            head.extensions.insert(extension);
            Ok(head)
        })
    }

    /// Get a reference to the extensions for this response builder.
    ///
    /// If the builder has an error, this returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Response;
    /// let res = Response::builder().extension("My Extension").extension(5u32);
    /// let extensions = res.extensions_ref().unwrap();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    #[deprecated(note = "Please use Builder2, it will replace Builder")]
    pub fn extensions_ref(&self) -> Option<&Extensions> {
        self.inner.as_ref().ok().map(|h| &h.extensions)
    }

    /// Get a mutable reference to the extensions for this response builder.
    ///
    /// If the builder has an error, this returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Response;
    /// let mut res = Response::builder().extension("My Extension");
    /// let mut extensions = res.extensions_mut().unwrap();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// extensions.insert(5u32);
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    #[deprecated(note = "Please use Builder2, it will replace Builder")]
    pub fn extensions_mut(&mut self) -> Option<&mut Extensions> {
        self.inner.as_mut().ok().map(|h| &mut h.extensions)
    }

    /// "Consumes" this builder, using the provided `body` to return a
    /// constructed `Response`.
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
    /// let response = Response::builder()
    ///     .body(())
    ///     .unwrap();
    /// ```
    #[deprecated(note = "Please use Builder2, it will replace Builder")]
    pub fn body<T>(self, body: T) -> Result<Response<T>> {
        self.inner.map(move |head| {
            Response {
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
    /// Creates a new default instance of `Builder` to construct either a
    /// `Head` or a `Response`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let response = response::Builder2::new()
    ///     .body(());
    /// ```
    #[inline]
    pub fn new() -> Builder2 {
        Builder2::default()
    }

    /// Set the HTTP status for this response.
    ///
    /// This function will configure the HTTP status code of the `Response` that
    /// will be returned from `Builder2::build`.
    ///
    /// By default this is `200`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let response = Response::builder2()
    ///     .status(StatusCode::NOT_FOUND)
    ///     .body(());
    /// ```
    pub fn status(mut self, status: StatusCode) -> Builder2 {
        self.inner.status = status.into();
        self
    }

    /// Try to Set the HTTP status for this response.
    ///
    /// This function will configure the HTTP status code of the `Response` that
    /// will be returned from `Builder2::build`, using a fallible conversion.
    /// If the conversion succeeds, the `Builder2` is updated with the status.
    /// If the conversion fails, the `Builder2` is discarded and the error is returned.
    ///
    /// By default the status is `200`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # fn main() -> Result<()> {
    /// let response = Response::builder2()
    ///     .try_status(404)?
    ///     .body(());
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_status<T>(self, status: T) -> Result<Builder2>
    where
        StatusCode: TryFrom<T>,
        crate::Error: From<<StatusCode as TryFrom<T>>::Error>,
    {
        use std::convert::TryInto;
        Ok(self.status(status.try_into()?))
    }

    /// Set the HTTP version for this response.
    ///
    /// This function will configure the HTTP version of the `Response` that
    /// will be returned from `Builder::build`.
    ///
    /// By default this is HTTP/1.1
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let response = Response::builder2()
    ///     .version(Version::HTTP_2)
    ///     .body(());
    /// ```
    pub fn version(mut self, version: Version) -> Builder2 {
        self.inner.version = version;
        self
    }

    /// Appends a header to this response builder.
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
    /// # fn main() -> Result<()> {
    /// let response = Response::builder2()
    ///     .try_header("Content-Type", "text/html")?
    ///     .try_header("X-Custom-Foo", "bar")?
    ///     .try_header("content-length", 0)?
    ///     .body(());
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_header<K, V>(self, key: K, value: V) -> Result<Builder2>
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<crate::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<crate::Error>,
    {
        let name = <HeaderName as TryFrom<K>>::try_from(key).map_err(Into::into)?;
        let value = <HeaderValue as TryFrom<V>>::try_from(value).map_err(Into::into)?;
        Ok(self.header(name, value))
    }

    /// Appends a header to this response builder.
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
    /// # use http::header::{CONTENT_TYPE, CONTENT_LENGTH};
    ///
    /// let response = Response::builder2()
    ///     .header(CONTENT_TYPE, HeaderValue::from_static("text/html"))
    ///     .header(HeaderName::from_static("x-custom-foo"), 17)
    ///     .header(CONTENT_LENGTH, 0)
    ///     .body(());
    /// ```
    pub fn header<K, V>(mut self, key: K, value: V) -> Builder2
    where
        HeaderName: From<K>,
        HeaderValue: From<V>,
    {
        self.inner
            .headers
            .append(HeaderName::from(key), HeaderValue::from(value));
        self
    }

    /// Get header on this response builder.
    ///
    /// When builder has error returns None.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Response;
    /// # use http::header::{ACCEPT, HeaderName, HeaderValue};
    /// let res = Response::builder2()
    ///     .header(ACCEPT, HeaderValue::from_static("text/html"))
    ///     .header(HeaderName::from_static("x-custom-foo"), 17);
    /// let headers = res.headers_ref();
    /// assert_eq!( headers["Accept"], "text/html" );
    /// assert_eq!( headers["X-Custom-Foo"], "17" );
    /// ```
    pub fn headers_ref(&self) -> &HeaderMap<HeaderValue> {
        &self.inner.headers
    }

    /// Get header on this response builder.
    /// when builder has error returns None
    ///
    /// # Example
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::HeaderValue;
    /// let mut res = Response::builder2();
    /// {
    ///   let headers = res.headers_mut();
    ///   headers.insert("Accept", HeaderValue::from_static("text/html"));
    ///   headers.insert("X-Custom-Foo", HeaderValue::from_static("bar"));
    /// }
    /// let headers = res.headers_ref();
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
    /// let response = Response::builder()
    ///     .extension("My Extension")
    ///     .body(())
    ///     .unwrap();
    ///
    /// assert_eq!(response.extensions().get::<&'static str>(),
    ///            Some(&"My Extension"));
    /// ```
    pub fn extension<T>(mut self, extension: T) -> Builder2
    where
        T: Any + Send + Sync + 'static,
    {
        self.inner.extensions.insert(extension);
        self
    }

    /// Get a reference to the extensions for this response builder.
    ///
    /// If the builder has an error, this returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Response;
    /// let req = Response::builder().extension("My Extension").extension(5u32);
    /// let extensions = req.extensions_ref().unwrap();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    pub fn extensions_ref(&self) -> &Extensions {
        &self.inner.extensions
    }

    /// Get a mutable reference to the extensions for this response builder.
    ///
    /// If the builder has an error, this returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Response;
    /// let mut req = Response::builder().extension("My Extension");
    /// let mut extensions = req.extensions_mut().unwrap();
    /// assert_eq!(extensions.get::<&'static str>(), Some(&"My Extension"));
    /// extensions.insert(5u32);
    /// assert_eq!(extensions.get::<u32>(), Some(&5u32));
    /// ```
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.inner.extensions
    }

    /// "Consumes" this builder, using the provided `body` to return a
    /// constructed `Response`.
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
    /// let response = Response::builder()
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn body<T>(self, body: T) -> Response<T> {
        Response {
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
    use super::Response;

    #[test]
    fn it_can_map_a_body_from_one_type_to_another() {
        #[allow(deprecated)]
        let response = Response::builder().body("some string").unwrap();
        let mapped_response = response.map(|s| {
            assert_eq!(s, "some string");
            123u32
        });
        assert_eq!(mapped_response.body(), &123u32);
    }

    #[test]
    fn it_can_map_a_body_from_one_type_to_another_2() {
        let response = Response::builder2().body("some string");
        let mapped_response = response.map(|s| {
            assert_eq!(s, "some string");
            123u32
        });
        assert_eq!(mapped_response.body(), &123u32);
    }
}
