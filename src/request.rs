//! HTTP request types.

use Uri;
use header::{HeaderMap, HeaderValue, HeaderMapKey};
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
    /// The request's method
    pub method: Method,

    /// The request's URI
    pub uri: Uri,

    /// The request's version
    pub version: Version,

    /// The request's headers
    pub headers: HeaderMap<HeaderValue>,

    _priv: (),
}

/// An HTTP request head builder
///
/// This type can be used to construct an instance of `Head` through a
/// builder-like pattern.
#[derive(Debug)]
pub struct HeadBuilder {
    head: Option<Head>,
}

impl Request<()> {
    /// Creates a new builder-style object to manufacture a `Request`
    ///
    /// This method returns an instance of `HeadBuilder` which can be used to
    /// create both the `Head` of a request or the `Request` itself.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use std::error::Error;
    /// # fn foo() -> Result<(), Box<Error>> {
    /// let request = Request::builder()
    ///     .method(method::GET)
    ///     .uri("https://www.rust-lang.org/".parse()?)
    ///     .header("X-Custom-Foo", "Bar".parse()?)
    ///     .request(());
    /// # Ok(())
    /// # }
    /// # fn main() {}
    /// ```
    pub fn builder() -> HeadBuilder {
        HeadBuilder::new()
    }
}

impl<T> Request<T> {
    /// Creates a new `Request` with the given head and body
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let head = request::Head::default();
    /// let request = Request::from_parts(head, "hello world");
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
    /// # use http::*;
    /// let request: Request<()> = Request::default();
    /// assert_eq!(*request.method(), method::GET);
    /// ```
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
    /// *request.method_mut() = method::PUT;
    /// assert_eq!(*request.method(), method::PUT);
    /// ```
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
    /// assert_eq!(request.version(), version::HTTP_11);
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
    /// let mut request: Request<()> = Request::default();
    /// *request.version_mut() = version::HTTP_2;
    /// assert_eq!(request.version(), version::HTTP_2);
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
    /// let request: Request<()> = Request::default();
    /// assert!(request.headers().is_empty());
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
    /// let mut request: Request<()> = Request::default();
    /// request.headers_mut().insert("hello", HeaderValue::from_static("world"));
    /// assert!(!request.headers().is_empty());
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
    /// let request: Request<String> = Request::default();
    /// assert!(request.body().is_empty());
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
    /// let mut request: Request<String> = Request::default();
    /// request.body_mut().push_str("hello world");
    /// assert!(!request.body().is_empty());
    /// ```
    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }

    /// Consumes the request returning the head and body parts.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request: Request<()> = Request::default();
    /// let (head, body) = request.into_parts();
    /// let request::Head { method, .. } = head;
    /// assert_eq!(method, method::GET);
    /// ```
    pub fn into_parts(self) -> (Head, T) {
        (self.head, self.body)
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

impl Head {
    /// Creates a new default instance of `Head`
    ///
    /// The returned `Head` has a GET method, a `/` URI, an HTTP/1.1 version,
    /// and no headers.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let mut head = request::Head::new();
    /// head.method = method::POST;
    /// ```
    pub fn new() -> Head {
        Head::default()
    }

    /// Creates a new instance of `HeadBuilder` which can be used to construct a
    /// `Head` with the builder pattern.
    ///
    /// # Examples
    /// ```
    /// # use http::*;
    /// # use std::error::Error;
    /// # fn foo() -> Result<(), Box<Error>> {
    /// let request = request::Head::builder()
    ///     .method(method::GET)
    ///     .uri("https://www.rust-lang.org/".parse()?)
    ///     .header("X-Custom-Foo", "Bar".parse()?)
    ///     .request(());
    /// # Ok(())
    /// # }
    /// # fn main() {}
    /// ```
    pub fn builder() -> HeadBuilder {
        HeadBuilder::default()
    }
}

impl HeadBuilder {
    /// Creates a new default instance of `Head`
    ///
    /// The returned `Head` has a GET method, a `/` URI, an HTTP/1.1 version,
    /// and no headers.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let mut head = request::Head::new();
    /// head.method = method::POST;
    /// ```
    pub fn new() -> HeadBuilder {
        HeadBuilder::default()
    }

    /// Set the HTTP method for this request.
    ///
    /// This function will configure the HTTP method of the `Request` that will
    /// be returned from `HeadBuilder::build`.
    ///
    /// By default this is `GET`.
    ///
    /// # Panics
    ///
    /// This method will panic if the `build` or `request` method has already
    /// been called on this builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let head = request::Head::builder()
    ///     .method(method::POST)
    ///     .build();
    /// ```
    pub fn method(&mut self, method: Method) -> &mut HeadBuilder {
        self.head().method = method;
        self
    }

    /// Set the URI for this request.
    ///
    /// This function will configure the URI of the `Request` that will
    /// be returned from `HeadBuilder::build`.
    ///
    /// By default this is `/`.
    ///
    /// # Panics
    ///
    /// This method will panic if the `build` or `request` method has already
    /// been called on this builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let head = request::Head::builder()
    ///     .uri("https://www.rust-lang.org/".parse().unwrap())
    ///     .build();
    /// ```
    pub fn uri(&mut self, uri: Uri) -> &mut HeadBuilder {
        self.head().uri = uri;
        self
    }

    /// Set the HTTP version for this request.
    ///
    /// This function will configure the HTTP version of the `Request` that
    /// will be returned from `HeadBuilder::build`.
    ///
    /// By default this is HTTP/1.1
    ///
    /// # Panics
    ///
    /// This method will panic if the `build` or `request` method has already
    /// been called on this builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// use http::version::HTTP_2;
    ///
    /// let head = request::Head::builder()
    ///     .version(HTTP_2)
    ///     .build();
    /// ```
    pub fn version(&mut self, version: Version) -> &mut HeadBuilder {
        self.head().version = version;
        self
    }

    /// Appends a header to this request builder.
    ///
    /// This function will append the provided key/value as a header to the
    /// internal `HeaderMap` being constructed. Essentially this is equivalent
    /// to calling `HeaderMap::append`.
    ///
    /// # Panics
    ///
    /// This method will panic if the `build` or `request` method has already
    /// been called on this builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::HeaderValue;
    ///
    /// let head = request::Head::builder()
    ///     .header("Accept", HeaderValue::from_static("text/html"))
    ///     .header("X-Custom-Foo", HeaderValue::from_static("bar"))
    ///     .build();
    /// ```
    pub fn header<K>(&mut self, key: K, value: HeaderValue) -> &mut HeadBuilder
        where K: HeaderMapKey,
    {
        self.head().headers.append(key, value);
        self
    }

    /// Appends a list of headersc to this request builder.
    ///
    /// This function will append the provided key/value pairs to the internal
    /// `HeaderMap` being constructed. Essentially this is equivalent to calling
    /// `HeaderMap::append` a number of times.
    ///
    /// # Panics
    ///
    /// This method will panic if the `build` or `request` method has already
    /// been called on this builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::HeaderValue;
    ///
    /// let mut headers = vec![
    ///     ("X-Custom-Foo", HeaderValue::from_static("bar")),
    /// ];
    ///
    /// if needs_custom_bar_header() {
    ///     headers.push(("X-Custom-Bar", HeaderValue::from_static("another")));
    /// }
    ///
    /// let head = request::Head::builder()
    ///     .headers(headers)
    ///     .build();
    /// # fn needs_custom_bar_header() -> bool { true }
    /// ```
    pub fn headers<I, K>(&mut self, headers: I) -> &mut HeadBuilder
        where I: IntoIterator<Item = (K, HeaderValue)>,
              K: HeaderMapKey,
    {
        for (key, value) in headers {
            self.header(key, value);
        }
        self
    }

    /// Clears all headers contained in this builder.
    ///
    /// This function will clear out all stored headers in this `Head` builder,
    /// erasing all previously configured values through `header` or `headers`.
    ///
    /// # Panics
    ///
    /// This method will panic if the `build` or `request` method has already
    /// been called on this builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// # use http::header::HeaderValue;
    /// let head = request::Head::builder()
    ///     .header("Accept", HeaderValue::from_static("text/html"))
    ///     .header("X-Custom-Foo", HeaderValue::from_static("bar"))
    ///     .headers_clear()
    ///     .header("X-Custom-Bar", HeaderValue::from_static("foo"))
    ///     .build();
    /// ```
    pub fn headers_clear(&mut self) -> &mut HeadBuilder {
        self.head().headers.clear();
        self
    }

    /// "Consumes" this builder, returning the constructed `Head`.
    ///
    /// # Panics
    ///
    /// This method will panic if the `build` or `request` method has already
    /// been called on this builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let head = request::Head::builder().build();
    /// ```
    pub fn build(&mut self) -> Head {
        self.head.take().expect("cannot re-build a builder after it's been used")
    }

    /// "Consumes" this builder, using the provided `body` to return a
    /// constructed `Request`.
    ///
    /// # Panics
    ///
    /// This method will panic if the `build` or `request` method has already
    /// been called on this builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let request = request::Head::builder().request(());
    /// ```
    pub fn request<T>(&mut self, body: T) -> Request<T> {
        Request {
            head: self.build(),
            body: body,
        }
    }

    fn head(&mut self) -> &mut Head {
        self.head.as_mut().expect("cannot configure a builder after it's been used")
    }
}

impl Default for HeadBuilder {
    fn default() -> HeadBuilder {
        HeadBuilder {
            head: Some(Head::default()),
        }
    }
}
