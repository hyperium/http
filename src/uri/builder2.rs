use std::convert::{TryFrom, TryInto};

use super::{Authority, PathAndQuery, Scheme};
use crate::Uri;

/// A builder for `Uri`s.
///
/// This type can be used to construct an instance of `Uri`
/// through a builder pattern.
///
/// The [Scheme], [Authority], and [PathAndQuery] can be set on the
/// builder, either directly or with fallible conversion.
///
/// # Examples
///
/// ```
/// # use http::*;
/// let uri = uri::Builder2::new()
///     .scheme(uri::Scheme::HTTPS)
///     .authority(uri::Authority::from_static("hyper.rs"))
///     .path_and_query(uri::PathAndQuery::from_static("/guides/client/basic/"))
///     .build();
/// assert_eq!(uri.to_string(), "https://hyper.rs/guides/client/basic/");
/// ```
///
/// ```
/// # use http::*;
/// # fn main() -> Result<()> {
/// let uri = uri::Builder2::new()
///     .try_scheme("https")?
///     .try_authority("hyper.rs")?
///     .try_path_and_query("/guides/client/basic/")?
///     .build();
/// assert_eq!(uri.to_string(), "https://hyper.rs/guides/client/basic/");
/// # Ok(())
/// # }
/// ```
///
/// It is possible to build an Uri with only the authority or only the
/// path and query part.
/// Invalid combinations does not have the `build` method, and will
/// not compile.
///
/// ```
/// # use http::*;
/// let uri = uri::Builder2::new()
///     .authority(uri::Authority::from_static("hyper.rs"))
///     .build();
/// assert_eq!(uri.to_string(), "hyper.rs");
/// ```
///
/// ```
/// # use http::*;
/// let uri = uri::Builder2::new()
///     .path_and_query(uri::PathAndQuery::from_static("/2020/page.html"))
///     .build();
/// assert_eq!(uri.to_string(), "/2020/page.html");
/// ```
#[derive(Debug, Default)]
pub struct Builder2<Parts = ()> {
    parts: Parts,
}

macro_rules! setter {
    ($field:ident, $try_field:ident, $type:ty, $rettype:ty, $retval:expr, $doc:expr) => {
        #[doc = $doc]
        pub fn $field(self, $field: $type) -> Builder2<$rettype> {
            Builder2::<$rettype> { parts: $retval(self.parts, $field) }
        }
    }
}
macro_rules! try_setter {
    ($field:ident, $try_field:ident, $type:ty, $rettype:ty, $doc:expr) => {
        #[doc = $doc]
        pub fn $try_field<T>(self, $field: T) -> Result<Builder2<$rettype>, crate::Error>
        where
            $type: TryFrom<T>,
            <$type as TryFrom<T>>::Error: Into<crate::Error>,
        {
            Ok(self.$field($field.try_into().map_err(Into::into)?))
        }
    }
}

macro_rules! methods {
    ($field:ident, $try_field:ident, $type:ty, $rettype:ty, $retval:expr) => {
        setter!(
            $field, $try_field, $type, $rettype, $retval,
            concat!("Set ", stringify!($type), " on this builder.")
        );
        try_setter!(
            $field, $try_field, $type, $rettype,
            concat!("Set ", stringify!($type), " on this builder with fallible conversion.")
        );
    }
}

impl Builder2 {
    /// Creates a new default instance of `Builder2` to construct a `Uri`.
    ///
    /// See also [Uri::builder2].
    #[inline]
    pub fn new() -> Builder2<()> {
        Builder2::default()
    }

    methods!(scheme, try_scheme, Scheme, Scheme, |_, s| s);
    methods!(authority, try_authority, Authority, Authority, |_, a| a);
    methods!(path_and_query, try_path_and_query, PathAndQuery, PathAndQuery, |_, pq| pq);

    /// Consumes this builder, and returns a valid `Uri` from
    /// the configured pieces.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let uri = Uri::builder2()
    ///     .build();
    /// assert_eq!(uri.to_string(), "");
    /// ```
    pub fn build(self) -> Uri {
        use super::scheme::Scheme2;
        Uri {
            scheme: Scheme {
                inner: Scheme2::None,
            },
            authority: Authority::empty(),
            path_and_query: PathAndQuery::empty(),
        }
    }
}


impl Builder2<Scheme> {
    methods!(scheme, try_scheme, Scheme, Scheme, |_, s| s);
    methods!(authority, try_authority, Authority, (Scheme, Authority), (|p, a| (p, a)));
    methods!(path_and_query, try_path_and_query, PathAndQuery, (Scheme, PathAndQuery), |p, pq| (p, pq));
}

impl Builder2<Authority> {
    methods!(scheme, try_scheme, Scheme, (Scheme, Authority), |a, s| (s, a));
    methods!(authority, try_authority, Authority, Authority, |_, a| a);
    methods!(path_and_query, try_path_and_query, PathAndQuery, (Authority, PathAndQuery), |a, pq| (a, pq));

    /// Consumes this builder, and returns a valid `Uri` from
    /// the configured pieces.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let uri = Uri::builder2()
    ///     .authority(uri::Authority::from_static("hyper.rs"))
    ///     .build();
    /// assert_eq!(uri.to_string(), "hyper.rs");
    /// ```
    pub fn build(self) -> Uri {
        use super::scheme::Scheme2;
        Uri {
            scheme: Scheme {
                inner: Scheme2::None,
            },
            authority: self.parts,
            path_and_query: PathAndQuery::empty(),
        }
    }
}

impl Builder2<PathAndQuery> {
    methods!(scheme, try_scheme, Scheme, (Scheme, PathAndQuery), |p, s| (s, p));
    methods!(authority, try_authority, Authority, (Authority, PathAndQuery), (|p, a| (a, p)));
    methods!(path_and_query, try_path_and_query, PathAndQuery, PathAndQuery, |_, pq| pq);

    /// Consumes this builder, and returns a valid `Uri` from
    /// the configured pieces.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let uri = uri::Builder2::new()
    ///     .path_and_query(uri::PathAndQuery::from_static("/2020/page.html"))
    ///     .build();
    /// assert_eq!(uri.to_string(), "/2020/page.html");
    /// ```
    pub fn build(self) -> Uri {
        use super::scheme::Scheme2;
        Uri {
            scheme: Scheme {
                inner: Scheme2::None,
            },
            authority: Authority::empty(),
            path_and_query: self.parts,
        }
    }
}

impl Builder2<(Scheme, Authority)> {
    methods!(scheme, try_scheme, Scheme, (Scheme, Authority), |p: (_,_), s| (s, p.1));
    methods!(authority, try_authority, Authority, (Scheme, Authority), |p: (_,_), a| (p.0, a));
    methods!(path_and_query, try_path_and_query, PathAndQuery, (Scheme, Authority, PathAndQuery), |p: (_,_), pq| (p.0, p.1, pq));
}

impl Builder2<(Scheme, PathAndQuery)> {
    methods!(scheme, try_scheme, Scheme, (Scheme, PathAndQuery), |p: (_,_), s| (s, p.1));
    methods!(authority, try_authority, Authority, (Scheme, Authority, PathAndQuery), |p: (_,_), a| (p.0, a, p.1));
    methods!(path_and_query, try_path_and_query, PathAndQuery, (Scheme, PathAndQuery), |p: (_,_), pq| (p.0, pq));
}

impl Builder2<(Authority, PathAndQuery)> {
    methods!(scheme, try_scheme, Scheme, (Scheme, Authority, PathAndQuery), |p: (_,_), s| (s, p.0, p.1));
    methods!(authority, try_authority, Authority, (Authority, PathAndQuery), |p: (_,_), a| (a, p.1));
    methods!(path_and_query, try_path_and_query, PathAndQuery, (Authority, PathAndQuery), |p: (_,_), pq| (p.0, pq));
}

impl Builder2<(Scheme, Authority, PathAndQuery)> {
    methods!(scheme, try_scheme, Scheme, (Scheme, Authority, PathAndQuery), |p: (_,_,_), s| (s, p.1, p.2));
    methods!(authority, try_authority, Authority, (Scheme, Authority, PathAndQuery), |p: (_,_,_), a| (p.0, a, p.2));
    methods!(path_and_query, try_path_and_query, PathAndQuery, (Scheme, Authority, PathAndQuery), |p: (_,_,_), pq| (p.0, p.1, pq));

    /// Consumes this builder, and returns a valid `Uri` from
    /// the configured pieces.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::*;
    /// let uri = Uri::builder2()
    ///     .scheme(uri::Scheme::HTTPS)
    ///     .authority(uri::Authority::from_static("hyper.rs"))
    ///     .path_and_query(uri::PathAndQuery::from_static("/guides/client/basic/"))
    ///     .build();
    /// assert_eq!(uri.to_string(), "https://hyper.rs/guides/client/basic/");
    /// ```
    pub fn build(self) -> Uri {
        Uri {
            scheme: self.parts.0,
            authority: self.parts.1,
            path_and_query: self.parts.2,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn build_from_str() -> Result<(), crate::Error> {
        let uri = Builder2::new()
            .scheme(Scheme::HTTP)
            .try_authority("hyper.rs")?
            .try_path_and_query("/foo?a=1")?
            .build();
        assert_eq!(uri.scheme_str(), Some("http"));
        assert_eq!(uri.authority().unwrap().host(), "hyper.rs");
        assert_eq!(uri.path(), "/foo");
        assert_eq!(uri.query(), Some("a=1"));
        Ok(())
    }

    #[test]
    fn build_from_string() -> Result<(), crate::Error> {
        for i in 1..10 {
            let uri = Builder2::new()
                .try_path_and_query(format!("/foo?a={}", i))?
                .build();
            let expected_query = format!("a={}", i);
            assert_eq!(uri.path(), "/foo");
            assert_eq!(uri.query(), Some(expected_query.as_str()));
        }
        Ok(())
    }

    #[test]
    fn build_from_string_ref() -> Result<(), crate::Error> {
        for i in 1..10 {
            let p_a_q = format!("/foo?a={}", i);
            let uri = Builder2::new().try_path_and_query(&p_a_q)?.build();
            let expected_query = format!("a={}", i);
            assert_eq!(uri.path(), "/foo");
            assert_eq!(uri.query(), Some(expected_query.as_str()));
        }
        Ok(())
    }
}
