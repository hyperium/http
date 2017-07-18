//! HTTP response types.

use std::io;

use {Error, Result, HttpTryFrom};
use header::{HeaderMap, HeaderValue};
use header::HeaderMapKey;
use status::StatusCode;
use version::Version;

/// Represents an HTTP response
///
/// An HTTP response consists of a head and a potentially optional body. The body
/// component is generic, enabling arbitrary types to represent the HTTP body.
/// For example, the body could be `Vec<u8>`, a `Stream` of byte chunks, or a
/// value that has been deserialized.
#[derive(Debug, Default)]
pub struct Response<T> {
    head: Head,
    body: T,
}

/// An HTTP response head
///
/// The HTTP response head consists of a status, version, and a set of
/// header fields.
#[derive(Debug, Default)]
pub struct Head {
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
/// This type can be used to construct an instance of `Head` or `Response`
/// through a builder-like pattern.
#[derive(Debug)]
pub struct Builder {
    head: Option<Head>,
    err: Option<Error>,
}

impl Response<()> {
    /// Creates a new builder-style object to manufacture a `Response`
    ///
    /// This method returns an instance of `Builder` which can be used to
    /// create both the `Head` of a request or the `Response` itself.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let request = Response::builder()
    ///     .status(200)
    ///     .header("X-Custom-Foo", "Bar")
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn builder() -> Builder {
        Builder::new()
    }
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

    /// Returns a reference to the associated extensions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let response: Response<()> = Response::default();
    /// assert!(response.extensions().get::<i32>().is_none());
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
    /// let mut response: Response<()> = Response::default();
    /// response.extensions_mut().insert("hello");
    /// assert_eq!(response.extensions().get(), Some(&"hello"));
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

impl Head {
    /// Creates a new instance of `Builder` which can be used to construct a
    /// `Head` with the builder pattern.
    ///
    /// # Examples
    /// ```
    /// # use http::*;
    /// let response = response::Head::builder()
    ///     .status(200)
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
    /// `Head` or a `Response`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let head = response::Builder::new()
    ///     .status(200)
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
    /// let head = Response::builder()
    ///     .status(200)
    ///     .head()
    ///     .unwrap();
    /// ```
    pub fn status<T>(&mut self, status: T) -> &mut Builder
        where StatusCode: HttpTryFrom<T>,
    {
        if let Some(head) = head(&mut self.head, &self.err) {
            match StatusCode::try_from(status) {
                Ok(s) => head.status = s,
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
    /// let head = Response::builder()
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
    /// let head = Response::builder()
    ///     .header("Content-Type", "text/html")
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
    /// let head = Response::builder()
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
    /// let head = response::Head::builder()
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
    /// let request = Response::builder()
    ///     .body(())
    ///     .unwrap();
    /// ```
    pub fn body<T>(&mut self, body: T) -> Result<Response<T>> {
        Ok(Response {
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
