use bytes::Bytes;

use Error;
use header::{self, HeaderName, HeaderValue};
use method::{self, Method};
use sealed::Sealed;
use status::{self, StatusCode};
use uri::{self, Uri};

/// dox
pub trait HttpTryFrom<T>: Sized + Sealed {
    /// dox
    type Error: Into<Error>;

    #[doc(hidden)]
    fn try_from(t: T) -> Result<Self, Self::Error>;
}

macro_rules! reflexive {
    ($($t:ty,)*) => ($(
        impl HttpTryFrom<$t> for $t {
            type Error = Error;

            fn try_from(t: Self) -> Result<Self, Self::Error> {
                Ok(t)
            }
        }

        impl Sealed for $t {}
    )*)
}

reflexive! {
    Uri,
    Method,
    StatusCode,
    HeaderName,
    HeaderValue,
}

impl<'a> HttpTryFrom<&'a str> for Uri {
    type Error = uri::FromStrError;

    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        t.parse()
    }
}

impl HttpTryFrom<Bytes> for Uri {
    type Error = uri::FromStrError;

    fn try_from(t: Bytes) -> Result<Self, Self::Error> {
        Uri::try_from_shared(t)
    }
}

impl<'a> HttpTryFrom<&'a [u8]> for Method {
    type Error = method::FromBytesError;

    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        Method::from_bytes(t)
    }
}

impl<'a> HttpTryFrom<&'a str> for Method {
    type Error = method::FromBytesError;

    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        Method::try_from(t.as_bytes())
    }
}

impl<'a> HttpTryFrom<&'a [u8]> for StatusCode {
    type Error = status::FromStrError;

    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        StatusCode::from_bytes(t)
    }
}

impl<'a> HttpTryFrom<&'a str> for StatusCode {
    type Error = status::FromStrError;

    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        t.parse()
    }
}

impl HttpTryFrom<u16> for StatusCode {
    type Error = status::FromU16Error;

    fn try_from(t: u16) -> Result<Self, Self::Error> {
        StatusCode::from_u16(t)
    }
}

impl<'a> HttpTryFrom<&'a str> for HeaderValue {
    type Error = header::InvalidValueError;

    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        t.parse()
    }
}

impl<'a> HttpTryFrom<&'a [u8]> for HeaderValue {
    type Error = header::InvalidValueError;

    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        HeaderValue::try_from_bytes(t)
    }
}
