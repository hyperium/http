use super::{Authority, Parts, PathAndQuery, Scheme};
use crate::convert::{HttpTryFrom, HttpTryInto};
use crate::{Result, Uri};

/// A builder for `Uri`s.
///
/// This type can be used to construct an instance of `Uri`
/// through a builder pattern.
#[derive(Debug)]
pub struct Builder {
    parts: Result<Parts>,
}

impl Builder {
    /// Creates a new default instance of `Builder` to construct a `Uri`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let uri = uri::Builder::new()
    ///     .scheme("https")
    ///     .authority("hyper.rs")
    ///     .path_and_query("/")
    ///     .build()
    ///     .unwrap();
    /// ```
    #[inline]
    pub fn new() -> Builder {
        Builder::default()
    }

    /// Set the `Scheme` for this URI.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let mut builder = uri::Builder::new();
    /// builder.scheme("https");
    /// ```
    pub fn scheme<T>(self, scheme: T) -> Self
    where
        Scheme: HttpTryFrom<T>,
    {
        self.map(move |mut parts| {
            parts.scheme = Some(scheme.http_try_into()?);
            Ok(parts)
        })
    }

    /// Set the `Authority` for this URI.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let uri = uri::Builder::new()
    ///     .authority("tokio.rs")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn authority<T>(self, auth: T) -> Self
    where
        Authority: HttpTryFrom<T>,
    {
        self.map(move |mut parts| {
            parts.authority = Some(auth.http_try_into()?);
            Ok(parts)
        })
    }

    /// Set the `PathAndQuery` for this URI.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let uri = uri::Builder::new()
    ///     .path_and_query("/hello?foo=bar")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn path_and_query<T>(self, p_and_q: T) -> Self
    where
        PathAndQuery: HttpTryFrom<T>,
    {
        self.map(move |mut parts| {
            parts.path_and_query = Some(p_and_q.http_try_into()?);
            Ok(parts)
        })
    }

    /// Consumes this builder, and tries to construct a valid `Uri` from
    /// the configured pieces.
    ///
    /// # Errors
    ///
    /// This function may return an error if any previously configured argument
    /// failed to parse or get converted to the internal representation. For
    /// example if an invalid `scheme` was specified via `scheme("!@#%/^")`
    /// the error will be returned when this function is called rather than
    /// when `scheme` was called.
    ///
    /// Additionally, the various forms of URI require certain combinations of
    /// parts to be set to be valid. If the parts don't fit into any of the
    /// valid forms of URI, a new error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    ///
    /// let uri = Uri::builder()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn build(self) -> Result<Uri> {
        self
            .parts
            .and_then(|parts| parts.http_try_into())
    }

    // private

    fn map<F>(self, func: F) -> Self
    where
        F: FnOnce(Parts) -> Result<Parts>,
    {

        Builder {
            parts: self.parts.and_then(func),
        }
    }
}

impl Default for Builder {
    #[inline]
    fn default() -> Builder {
        Builder {
            parts: Ok(Parts::default()),
        }
    }
}
