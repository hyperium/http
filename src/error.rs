use std::error;
use std::fmt;
use std::io;
use std::result;

use header;
use method;
use status;
use uri;

/// A generic "error" for HTTP connections
///
/// This error type is less specific than the error returned from other
/// functions in this crate, but all other errors can be converted to this
/// error. Consumers of this crate can typically consume and work with this form
/// of error for conversions with the `?` operator.
#[derive(Debug)]
pub struct Error {
    inner: ErrorKind,
}

/// A `Result` typedef to use with the `http::Error` type
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
enum ErrorKind {
    StatusU16(status::FromU16Error),
    StatusStr(status::FromStrError),
    MethodBytes(method::FromBytesError),
    UriStr(uri::FromStrError),
    HeaderInvalid(header::InvalidValueError),
    HeaderStr(header::ToStrError),
    HeaderNameBytes(header::FromBytesError),
    HeaderNameStr(header::FromStrError),
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", error::Error::description(self))?;
        match self.inner {
            ErrorKind::Io(ref e) => write!(f, ": {}", e),
            _ => Ok(())
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        use self::ErrorKind::*;

        match self.inner {
            StatusU16(ref e) => e.description(),
            StatusStr(ref e) => e.description(),
            MethodBytes(ref e) => e.description(),
            UriStr(ref e) => e.description(),
            HeaderInvalid(ref e) => e.description(),
            HeaderStr(ref e) => e.description(),
            HeaderNameBytes(ref e) => e.description(),
            HeaderNameStr(ref e) => e.description(),
            Io(_) => "I/O error"
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        use self::ErrorKind::*;

        match self.inner {
            Io(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<status::FromU16Error> for Error {
    fn from(err: status::FromU16Error) -> Error {
        Error { inner: ErrorKind::StatusU16(err) }
    }
}

impl From<status::FromStrError> for Error {
    fn from(err: status::FromStrError) -> Error {
        Error { inner: ErrorKind::StatusStr(err) }
    }
}

impl From<method::FromBytesError> for Error {
    fn from(err: method::FromBytesError) -> Error {
        Error { inner: ErrorKind::MethodBytes(err) }
    }
}

impl From<uri::FromStrError> for Error {
    fn from(err: uri::FromStrError) -> Error {
        Error { inner: ErrorKind::UriStr(err) }
    }
}

impl From<header::InvalidValueError> for Error {
    fn from(err: header::InvalidValueError) -> Error {
        Error { inner: ErrorKind::HeaderInvalid(err) }
    }
}

impl From<header::ToStrError> for Error {
    fn from(err: header::ToStrError) -> Error {
        Error { inner: ErrorKind::HeaderStr(err) }
    }
}

impl From<header::FromBytesError> for Error {
    fn from(err: header::FromBytesError) -> Error {
        Error { inner: ErrorKind::HeaderNameBytes(err) }
    }
}

impl From<header::FromStrError> for Error {
    fn from(err: header::FromStrError) -> Error {
        Error { inner: ErrorKind::HeaderNameStr(err) }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error { inner: ErrorKind::Io(err) }
    }
}
