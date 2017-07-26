//! HTTP request types.

use std::any::Any;

use {Uri, Error, Result, HttpTryFrom, Extensions};
use header::{HeaderMap, HeaderValue, HeaderMapKey};
use method::Method;
use version::Version;

/// Represents an HTTP request.
///
/// An HTTP request consists of a head and a potentially optional body. The body
/// component is generic, enabling arbitrary types to represent the HTTP body.
/// For example, the body could be `Vec<u8>`, a `Stream` of byte chunks, or a
/// value that has been deserialized.
#[derive(Debug)]
pub struct Request<T> {
    head: Parts,
    body: T,
}

/// Component parts of an HTTP `Request`
///
/// The HTTP request head consists of a method, uri, version, and a set of
/// header fields.
#[derive(Debug)]
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
    head: Option<Parts>,
    err: Option<Error>,
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
    pub fn builder() -> Builder {
        Builder::new()
    }
}

impl<T> Request<T> {
    /// Creates a new blank `Request` with the body
    ///
    /// The component ports of this request will be set to their default, e.g.
    /// the GET method, no headers, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Request::new("hello world");
    ///
    /// assert_eq!(*request.method(), method::GET);
    /// assert_eq!(*request.body(), "hello world");
    /// ```
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
    /// parts.method = method::POST;
    ///
    /// let request = Request::from_parts(parts, body);
    /// ```
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
    /// let request = Request::new(());
    /// let (parts, body) = request.into_parts();
    /// assert_eq!(parts.method, method::GET);
    /// ```
    pub fn into_parts(self) -> (Parts, T) {
        (self.head, self.body)
    }
}

impl<T: Default> Default for Request<T> {
    fn default() -> Request<T> {
        Request::new(T::default())
    }
}

impl Parts {
    /// Creates a new default instance of `Parts`
    fn new() -> Parts {
        Parts{
            method: Method::default(),
            uri: Uri::default(),
            version: Version::default(),
            headers: HeaderMap::default(),
            extensions: Extensions::default(),
            _priv: (),
        }
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
    /// let req = request::Builder::new()
    ///     .method("POST")
    ///     .body(())
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
    /// let req = Request::builder()
    ///     .method("POST")
    ///     .body(())
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
    /// let req = Request::builder()
    ///     .uri("https://www.rust-lang.org/")
    ///     .body(())
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
    /// let req = Request::builder()
    ///     .version(HTTP_2)
    ///     .body(())
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
    /// let req = Request::builder()
    ///     .header("Accept", "text/html")
    ///     .header("X-Custom-Foo", "bar")
    ///     .body(())
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
    /// let req = Request::builder()
    ///     .headers(headers)
    ///     .body(())
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
    pub fn extension<T>(&mut self, extension: T) -> &mut Builder
        where T: Any + Send + Sync + 'static,
    {
        if let Some(head) = head(&mut self.head, &self.err) {
            head.extensions.insert(extension);
        }
        self
    }

    fn take_parts(&mut self) -> Result<Parts> {
        let ret = self.head.take().expect("cannot reuse request builder");
        if let Some(e) = self.err.take() {
            return Err(e)
        }
        Ok(ret)
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
    /// # Panics
    ///
    /// This method will panic if the builder is reused. The `body` function can
    /// only be called once.
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
            head: self.take_parts()?,
            body: body,
        })
    }
}

fn head<'a>(head: &'a mut Option<Parts>, err: &Option<Error>)
    -> Option<&'a mut Parts>
{
    if err.is_some() {
        return None
    }
    head.as_mut()
}

impl Default for Builder {
    fn default() -> Builder {
        Builder {
            head: Some(Parts::new()),
            err: None,
        }
    }
}
