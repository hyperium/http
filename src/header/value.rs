use bytes::Bytes;

use std::{char, cmp, fmt, str};
use std::error::Error;
use std::str::FromStr;

use ::convert::HttpTryFrom;

/// Represents an HTTP header field value.
///
/// In practice, HTTP header field values are usually valid ASCII. However, the
/// HTTP spec allows for a header value to contain opaque bytes as well. In this
/// case, the header field value is not able to be represented as a string.
///
/// To handle this, the `HeaderValue` is useable as a type and can be compared
/// with strings and implements `Debug`. A `to_str` fn is provided that returns
/// an `Err` if the header value contains non visible ascii characters.
#[derive(Clone, Hash)]
pub struct HeaderValue {
    inner: Bytes,
    is_sensitive: bool,
}

/// A possible error when converting a `HeaderValue` from a string or byte
/// slice.
#[derive(Debug)]
pub struct InvalidHeaderValue {
    _priv: (),
}

/// A possible error when converting a `HeaderValue` from a string or byte
/// slice.
#[derive(Debug)]
pub struct InvalidHeaderValueBytes(InvalidHeaderValue);

/// A possible error when converting a `HeaderValue` to a string representation.
///
/// Header field values may contain opaque bytes, in which case it is not
/// possible to represent the value as a string.
#[derive(Debug)]
pub struct ToStrError {
    _priv: (),
}

impl HeaderValue {
    /// Convert a static string to a `HeaderValue`
    ///
    /// This function will not perform any copying, however the string is
    /// checked to ensure that no invalid characters are present. Only visible
    /// ASCII characters (32-127) are permitted.
    ///
    /// # Panics
    ///
    /// This function panics if the argument contains invalid header value
    /// characters.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_static("hello");
    /// assert_eq!(val, "hello");
    /// ```
    #[inline]
    pub fn from_static(src: &'static str) -> HeaderValue {
        let bytes = src.as_bytes();
        for &b in bytes {
            if !is_visible_ascii(b) {
                panic!("invalid header value");
            }
        }

        HeaderValue {
            inner: Bytes::from_static(bytes),
            is_sensitive: false,
        }
    }

    /// Attempt to convert a string to a `HeaderValue`.
    ///
    /// If the argument contains invalid header value characters, an error is
    /// returned. Only visible ASCII characters (32-127) are permitted. Use
    /// `try_from_bytes` to create a `HeaderValue` that includes opaque octets
    /// (128-255).
    ///
    /// This function is intended to be replaced in the future by a `TryFrom`
    /// implementation once the trait is stabilized in std.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::try_from_str("hello").unwrap();
    /// assert_eq!(val, "hello");
    /// ```
    ///
    /// An invalid value
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::try_from_str("\n");
    /// assert!(val.is_err());
    /// ```
    #[inline]
    pub fn try_from_str(src: &str) -> Result<HeaderValue, InvalidHeaderValue> {
        HeaderValue::try_from(src)
    }

    /// Attempt to convert a byte slice to a `HeaderValue`.
    ///
    /// If the argument contains invalid header value bytes, an error is
    /// returned. Only byte values between 32 and 255 (inclusive) are permitted,
    /// excluding byte 127 (DEL).
    ///
    /// This function is intended to be replaced in the future by a `TryFrom`
    /// implementation once the trait is stabilized in std.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::try_from_bytes(b"hello\xfa").unwrap();
    /// assert_eq!(val, &b"hello\xfa"[..]);
    /// ```
    ///
    /// An invalid value
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::try_from_bytes(b"\n");
    /// assert!(val.is_err());
    /// ```
    #[inline]
    pub fn try_from_bytes(src: &[u8]) -> Result<HeaderValue, InvalidHeaderValue> {
        HeaderValue::try_from(src)
    }

    /// Attempt to convert a `Bytes` buffer to a `HeaderValue`.
    ///
    /// If the argument contains invalid header value bytes, an error is
    /// returned. Only byte values between 32 and 255 (inclusive) are permitted,
    /// excluding byte 127 (DEL).
    ///
    /// This function is intended to be replaced in the future by a `TryFrom`
    /// implementation once the trait is stabilized in std.
    #[inline]
    pub fn try_from_shared(src: Bytes) -> Result<HeaderValue, InvalidHeaderValueBytes> {
        HeaderValue::try_from(src).map_err(InvalidHeaderValueBytes)
    }

    fn try_from<T: AsRef<[u8]> + Into<Bytes>>(src: T) -> Result<HeaderValue, InvalidHeaderValue> {
        for &b in src.as_ref() {
            if !is_valid(b) {
                return Err(InvalidHeaderValue {
                    _priv: (),
                });
            }
        }
        Ok(HeaderValue {
            inner: src.into(),
            is_sensitive: false,
        })
    }

    /// Yields a `&str` slice if the `HeaderValue` only contains visible ASCII
    /// chars.
    ///
    /// This function will perform a scan of the header value, checking all the
    /// characters.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_static("hello");
    /// assert_eq!(val.to_str().unwrap(), "hello");
    /// ```
    pub fn to_str(&self) -> Result<&str, ToStrError> {
        let bytes = self.as_ref();

        for &b in bytes {
            if !is_visible_ascii(b) {
                return Err(ToStrError { _priv: () });
            }
        }

        unsafe { Ok(str::from_utf8_unchecked(bytes)) }
    }

    /// Returns the length of `self`.
    ///
    /// This length is in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_static("hello");
    /// assert_eq!(val.len(), 5);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.as_ref().len()
    }

    /// Returns true if the `HeaderValue` has a length of zero bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_static("");
    /// assert!(val.is_empty());
    ///
    /// let val = HeaderValue::from_static("hello");
    /// assert!(!val.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Converts a `HeaderValue` to a byte slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let val = HeaderValue::from_static("hello");
    /// assert_eq!(val.as_bytes(), b"hello");
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }

    /// Mark that the header value represents sensitive information.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let mut val = HeaderValue::from_static("my secret");
    ///
    /// val.set_sensitive(true);
    /// assert!(val.is_sensitive());
    ///
    /// val.set_sensitive(false);
    /// assert!(!val.is_sensitive());
    /// ```
    #[inline]
    pub fn set_sensitive(&mut self, val: bool) {
        self.is_sensitive = val;
    }

    /// Returns `true` if the value represents sensitive data.
    ///
    /// Sensitive data could represent passwords or other data that should not
    /// be stored on disk or in memory. This setting can be used by components
    /// like caches to avoid storing the value. HPACK encoders must set the
    /// header field to never index when `is_sensitive` returns true.
    ///
    /// Note that sensitivity is not factored into equality or ordering.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::header::HeaderValue;
    /// let mut val = HeaderValue::from_static("my secret");
    ///
    /// val.set_sensitive(true);
    /// assert!(val.is_sensitive());
    ///
    /// val.set_sensitive(false);
    /// assert!(!val.is_sensitive());
    /// ```
    #[inline]
    pub fn is_sensitive(&self) -> bool {
        self.is_sensitive
    }
}

impl AsRef<[u8]> for HeaderValue {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl fmt::Debug for HeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("HeaderValue")
            .field("value", &EscapeBytes(self.as_ref()))
            .field("is_sensitive", &self.is_sensitive)
            .finish()
    }
}

impl FromStr for HeaderValue {
    type Err = InvalidHeaderValue;

    #[inline]
    fn from_str(s: &str) -> Result<HeaderValue, Self::Err> {
        HeaderValue::try_from_str(s)
    }
}

impl From<HeaderValue> for Bytes {
    #[inline]
    fn from(value: HeaderValue) -> Bytes {
        value.inner
    }
}

impl<'a> HttpTryFrom<&'a str> for HeaderValue {
    type Error = InvalidHeaderValue;

    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        t.parse()
    }
}

impl<'a> HttpTryFrom<&'a [u8]> for HeaderValue {
    type Error = InvalidHeaderValue;

    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        HeaderValue::try_from_bytes(t)
    }
}

impl HttpTryFrom<Bytes> for HeaderValue {
    type Error = InvalidHeaderValueBytes;

    #[inline]
    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        HeaderValue::try_from_shared(bytes)
    }
}

struct EscapeBytes<'a>(&'a [u8]);

impl<'a> fmt::Debug for EscapeBytes<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &b in self.0 {
            if is_visible_ascii(b) {
                let ch = unsafe { char::from_u32_unchecked(b as u32) };
                write!(f, "{}", ch)?;
            } else {
                write!(f, "\\x{:x}", b)?;
            }
        }

        Ok(())
    }
}

fn is_visible_ascii(b: u8) -> bool {
    b >= 32 && b < 127
}

fn is_valid(b: u8) -> bool {
    b >= 32 && b != 127
}

impl fmt::Display for InvalidHeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description().fmt(f)
    }
}

impl Error for InvalidHeaderValue {
    fn description(&self) -> &str {
        "failed to parse header value"
    }
}

impl fmt::Display for InvalidHeaderValueBytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for InvalidHeaderValueBytes {
    fn description(&self) -> &str {
        self.0.description()
    }
}

impl fmt::Display for ToStrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description().fmt(f)
    }
}

impl Error for ToStrError {
    fn description(&self) -> &str {
        "failed to convert header to a str"
    }
}

// ===== PartialEq / PartialOrd =====

impl PartialEq for HeaderValue {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        self.inner == other.inner
    }
}

impl Eq for HeaderValue {}

impl PartialOrd for HeaderValue {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl Ord for HeaderValue {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl PartialEq<str> for HeaderValue {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.inner == other.as_bytes()
    }
}

impl PartialEq<[u8]> for HeaderValue {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.inner == other
    }
}

impl PartialOrd<str> for HeaderValue {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<cmp::Ordering> {
        (*self.inner).partial_cmp(other.as_bytes())
    }
}

impl PartialOrd<[u8]> for HeaderValue {
    #[inline]
    fn partial_cmp(&self, other: &[u8]) -> Option<cmp::Ordering> {
        (*self.inner).partial_cmp(other)
    }
}

impl PartialEq<HeaderValue> for str {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        *other == *self
    }
}

impl PartialEq<HeaderValue> for [u8] {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        *other == *self
    }
}

impl PartialOrd<HeaderValue> for str {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

impl PartialOrd<HeaderValue> for [u8] {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        self.partial_cmp(other.as_bytes())
    }
}

impl PartialEq<String> for HeaderValue {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        *self == &other[..]
    }
}

impl PartialOrd<String> for HeaderValue {
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(other.as_bytes())
    }
}

impl PartialEq<HeaderValue> for String {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        *other == *self
    }
}

impl PartialOrd<HeaderValue> for String {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

impl<'a> PartialEq<HeaderValue> for &'a HeaderValue {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        **self == *other
    }
}

impl<'a> PartialOrd<HeaderValue> for &'a HeaderValue {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

impl<'a, T: ?Sized> PartialEq<&'a T> for HeaderValue
    where HeaderValue: PartialEq<T>
{
    #[inline]
    fn eq(&self, other: &&'a T) -> bool {
        *self == **other
    }
}

impl<'a, T: ?Sized> PartialOrd<&'a T> for HeaderValue
    where HeaderValue: PartialOrd<T>
{
    #[inline]
    fn partial_cmp(&self, other: &&'a T) -> Option<cmp::Ordering> {
        self.partial_cmp(*other)
    }
}

impl<'a> PartialEq<HeaderValue> for &'a str {
    #[inline]
    fn eq(&self, other: &HeaderValue) -> bool {
        *other == *self
    }
}

impl<'a> PartialOrd<HeaderValue> for &'a str {
    #[inline]
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

#[test]
fn test_try_from() {
    HeaderValue::try_from(vec![127]).unwrap_err();
}
