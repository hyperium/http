//! HTTP request types.

use std::io;

use {Uri, Error, Result, HttpTryFrom};
use header::{HeaderMap, HeaderValue, HeaderMapKey};
use method::Method;
use version::Version;

/// Represents an HTTP request.
///
/// An HTTP request consists of a head and a potentially optional body. The body
/// component is generic, enabling arbitrary types to represent the HTTP body.
/// For example, the body could be `Vec<u8>`, a `Stream` of byte chunks, or a
/// value that has been deserialized.
#[derive(Debug, Default)]
pub struct Request<T> {
    head: Head,
    body: T,
}

/// An HTTP request head
///
/// The HTTP request head consists of a method, uri, version, and a set of
/// header fields.
#[derive(Debug, Default)]
pub struct Head {
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
/// This type can be used to construct an instance of `Head` or `Request`
/// through a builder-like pattern.
#[derive(Debug)]
pub struct Builder {
    head: Option<Head>,
    err: Option<Error>,
}

impl Request<()> {
    /// Creates a new builder-style object to manufacture a `Request`
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create both the `Head` of a request or the `Request` itself.
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
    pub fn builder() -> Builder {
        Builder::new()
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


    /// Returns a reference to the associated extensions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request: Request<()> = Request::default();
    /// assert!(request.extensions().get::<i32>().is_none());
    /// ```
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

    /// Creates a new instance of `Builder` which can be used to construct a
    /// `Head` with the builder pattern.
    ///
    /// # Examples
    /// ```
    /// # use http::*;
    /// let head = request::Head::builder()
    ///     .method("GET")
    ///     .uri("https://www.rust-lang.org")
    ///     .header("X-Custom-Foo", "Bar")
    ///     .head()
    ///     .unwrap();
    /// ```
    pub fn builder() -> Builder {
        Builder::default()
    }
}

impl Builder {
    /// Creates a new default instance of `Builder` to construct either a
    /// `Head` or a `Request`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let head = request::Builder::new()
    ///     .method("POST")
    ///     .head()
    ///     .unwrap();
    /// ```
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
    /// let head = Request::builder()
    ///     .method("POST")
    ///     .head()
    ///     .unwrap();
    /// ```
    pub fn method<T>(&mut self, method: T) -> &mut Builder
        where Method: HttpTryFrom<T>,
    {
        if let Some(head) = head(&mut self.head, &self.err) {
            match Method::try_from(method) {
                Ok(s) => head.method = s,
                Err(e) => self.err = Some(e.into()),
            }
        }
        self
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
    /// let head = Request::builder()
    ///     .uri("https://www.rust-lang.org/")
    ///     .head()
    ///     .unwrap();
    /// ```
    pub fn uri<T>(&mut self, uri: T) -> &mut Builder
        where Uri: HttpTryFrom<T>,
    {
        if let Some(head) = head(&mut self.head, &self.err) {
            match Uri::try_from(uri) {
                Ok(s) => head.uri = s,
                Err(e) => self.err = Some(e.into()),
            }
        }
        self
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
    /// use http::version::HTTP_2;
    ///
    /// let head = Request::builder()
    ///     .version(HTTP_2)
    ///     .head()
    ///     .unwrap();
    /// ```
    pub fn version(&mut self, version: Version) -> &mut Builder {
        if let Some(head) = head(&mut self.head, &self.err) {
            head.version = version;
        }
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
    /// # use http::header::HeaderValue;
    ///
    /// let head = Request::builder()
    ///     .header("Accept", "text/html")
    ///     .header("X-Custom-Foo", "bar")
    ///     .head()
    ///     .unwrap();
    /// ```
    pub fn header<K, V>(&mut self, key: K, value: V) -> &mut Builder
        where K: HeaderMapKey,
              HeaderValue: HttpTryFrom<V>
    {
        if let Some(head) = head(&mut self.head, &self.err) {
            match HeaderValue::try_from(value) {
                Ok(value) => { head.headers.append(key, value); }
                Err(e) => self.err = Some(e.into()),
            }
        }
        self
    }

    /// Appends a list of headersc to this request builder.
    ///
    /// This function will append the provided key/value pairs to the internal
    /// `HeaderMap` being constructed. Essentially this is equivalent to calling
    /// `HeaderMap::append` a number of times.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let mut headers = vec![
    ///     ("X-Custom-Foo", "bar"),
    /// ];
    ///
    /// if needs_custom_bar_header() {
    ///     headers.push(("X-Custom-Bar", "another"));
    /// }
    ///
    /// let head = Request::builder()
    ///     .headers(headers)
    ///     .head()
    ///     .unwrap();
    /// # fn needs_custom_bar_header() -> bool { true }
    /// ```
    pub fn headers<I, K, V>(&mut self, headers: I) -> &mut Builder
        where I: IntoIterator<Item = (K, V)>,
              K: HeaderMapKey,
              HeaderValue: HttpTryFrom<V>,
    {
        for (key, value) in headers {
            self.header(key, value);
        }
        self
    }

    /// "Consumes" this builder, returning the constructed `Head`.
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
    /// let head = request::Head::builder()
    ///     .head()
    ///     .unwrap();
    /// ```
    pub fn head(&mut self) -> Result<Head> {
        if let Some(e) = self.err.take() {
            return Err(e)
        }
        self.head.take().ok_or_else(|| {
            io::Error::new(io::ErrorKind::Other, "cannot reuse `Builder`")
                .into()
        })
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
    pub fn body<T>(&mut self, body: T) -> Result<Request<T>> {
        Ok(Request {
            head: self.head()?,
            body: body,
        })
    }
}

fn head<'a>(head: &'a mut Option<Head>, err: &Option<Error>)
    -> Option<&'a mut Head>
{
    if err.is_some() {
        return None
    }
    head.as_mut()
}

impl Default for Builder {
    fn default() -> Builder {
        Builder {
            head: Some(Head::default()),
            err: None,
        }
    }
}
