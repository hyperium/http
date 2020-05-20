use std::{
    convert::{TryFrom, TryInto},
    fmt::Write,
};

use super::{Authority, ErrorKind, InvalidUriParts, Parts, PathAndQuery, Scheme};
use crate::{byte_str::ByteStr, Uri};

/// A builder for `Uri`s.
///
/// This type can be used to construct an instance of `Uri`
/// through a builder pattern.
#[derive(Debug)]
pub struct Builder {
    parts: Result<Parts, crate::Error>,
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
        Scheme: TryFrom<T>,
        <Scheme as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.map(move |mut parts| {
            let scheme = scheme.try_into().map_err(Into::into)?;
            parts.scheme = Some(scheme);
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
        Authority: TryFrom<T>,
        <Authority as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.map(move |mut parts| {
            let auth = auth.try_into().map_err(Into::into)?;
            parts.authority = Some(auth);
            Ok(parts)
        })
    }

    /// Set the port number for URI, will be part of `Authority`
    pub fn port<P>(self, port: P) -> Self
    where
        P: Into<u16>,
    {
        let port: u16 = port.into();
        self.map(move |mut parts| {
            let prev_auth = match parts.authority.as_ref() {
                Some(auth) => {
                    if auth.port().is_some() {
                        return Err(InvalidUriParts::from(ErrorKind::InvalidPort).into());
                    }

                    auth.as_str()
                }
                None => "",
            };
            // 1 for ':', 5 for port number digists
            let mut auth = String::with_capacity(prev_auth.len() + 6);
            auth.push_str(prev_auth);
            auth.push(':');
            write!(&mut auth, "{}", port).expect("write to String failed");
            let data: ByteStr = auth.into();
            parts.authority = Some(Authority { data });
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
        PathAndQuery: TryFrom<T>,
        <PathAndQuery as TryFrom<T>>::Error: Into<crate::Error>,
    {
        self.map(move |mut parts| {
            let p_and_q = p_and_q.try_into().map_err(Into::into)?;
            parts.path_and_query = Some(p_and_q);
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
    pub fn build(self) -> Result<Uri, crate::Error> {
        let parts = self.parts?;
        Uri::from_parts(parts).map_err(Into::into)
    }

    // private

    fn map<F>(self, func: F) -> Self
    where
        F: FnOnce(Parts) -> Result<Parts, crate::Error>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_from_str() {
        let uri = Builder::new()
            .scheme(Scheme::HTTP)
            .authority("hyper.rs")
            .path_and_query("/foo?a=1")
            .build()
            .unwrap();
        assert_eq!(uri.scheme_str(), Some("http"));
        assert_eq!(uri.authority().unwrap().host(), "hyper.rs");
        assert_eq!(uri.path(), "/foo");
        assert_eq!(uri.query(), Some("a=1"));
    }

    #[test]
    fn build_from_string() {
        for i in 1..10 {
            let uri = Builder::new()
                .path_and_query(format!("/foo?a={}", i))
                .build()
                .unwrap();
            let expected_query = format!("a={}", i);
            assert_eq!(uri.path(), "/foo");
            assert_eq!(uri.query(), Some(expected_query.as_str()));
        }
    }

    #[test]
    fn build_from_string_ref() {
        for i in 1..10 {
            let p_a_q = format!("/foo?a={}", i);
            let uri = Builder::new().path_and_query(&p_a_q).build().unwrap();
            let expected_query = format!("a={}", i);
            assert_eq!(uri.path(), "/foo");
            assert_eq!(uri.query(), Some(expected_query.as_str()));
        }
    }
}
