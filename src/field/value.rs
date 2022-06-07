use bytes::{Bytes, BytesMut};

use std::convert::TryFrom;
use std::error::Error;
use std::fmt::Write;
use std::str::FromStr;
use std::{cmp, fmt, mem, str};

use crate::field::name::FieldName;

/// Represents an HTTP header field value.
///
/// In practice, HTTP header field values are usually valid ASCII. However, the
/// HTTP spec allows for a header value to contain opaque bytes as well. In this
/// case, the header field value is not able to be represented as a string.
///
/// To handle this, the `FieldValue` is useable as a type and can be compared
/// with strings and implements `Debug`. A `to_str` fn is provided that returns
/// an `Err` if the header value contains non visible ascii characters.
#[derive(Clone, Hash)]
pub struct FieldValue {
    inner: Bytes,
    is_sensitive: bool,
}

/// A possible error when converting a `FieldValue` from a string or byte
/// slice.
pub struct InvalidFieldValue {
    _priv: (),
}

/// A possible error when converting a `FieldValue` to a string representation.
///
/// Header field values may contain opaque bytes, in which case it is not
/// possible to represent the value as a string.
#[derive(Debug)]
pub struct ToStrError {
    _priv: (),
}

impl FieldValue {
    /// Convert a static string to a `FieldValue`.
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
    /// Until [Allow panicking in constants](https://github.com/rust-lang/rfcs/pull/2345)
    /// makes its way into stable, the panic message at compile-time is
    /// going to look cryptic, but should at least point at your header value:
    ///
    /// ```text
    /// error: any use of this value will cause an error
    ///   --> http/src/header/value.rs:67:17
    ///    |
    /// 67 |                 ([] as [u8; 0])[0]; // Invalid header value
    ///    |                 ^^^^^^^^^^^^^^^^^^
    ///    |                 |
    ///    |                 index out of bounds: the length is 0 but the index is 0
    ///    |                 inside `FieldValue::from_static` at http/src/header/value.rs:67:17
    ///    |                 inside `INVALID_HEADER` at src/main.rs:73:33
    ///    |
    ///   ::: src/main.rs:73:1
    ///    |
    /// 73 | const INVALID_HEADER: FieldValue = FieldValue::from_static("жsome value");
    ///    | ----------------------------------------------------------------------------
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::field::FieldValue;
    /// let val = FieldValue::from_static("hello");
    /// assert_eq!(val, "hello");
    /// ```
    #[inline]
    #[allow(unconditional_panic)] // required for the panic circumvention
    pub const fn from_static(src: &'static str) -> FieldValue {
        let bytes = src.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !is_visible_ascii(bytes[i]) {
                ([] as [u8; 0])[0]; // Invalid header value
            }
            i += 1;
        }

        FieldValue {
            inner: Bytes::from_static(bytes),
            is_sensitive: false,
        }
    }

    /// Attempt to convert a string to a `FieldValue`.
    ///
    /// If the argument contains invalid header value characters, an error is
    /// returned. Only visible ASCII characters (32-127) are permitted. Use
    /// `from_bytes` to create a `FieldValue` that includes opaque octets
    /// (128-255).
    ///
    /// This function is intended to be replaced in the future by a `TryFrom`
    /// implementation once the trait is stabilized in std.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::field::FieldValue;
    /// let val = FieldValue::from_str("hello").unwrap();
    /// assert_eq!(val, "hello");
    /// ```
    ///
    /// An invalid value
    ///
    /// ```
    /// # use http::field::FieldValue;
    /// let val = FieldValue::from_str("\n");
    /// assert!(val.is_err());
    /// ```
    #[inline]
    pub fn from_str(src: &str) -> Result<FieldValue, InvalidFieldValue> {
        FieldValue::try_from_generic(src, |s| Bytes::copy_from_slice(s.as_bytes()))
    }

    /// Converts a FieldName into a FieldValue
    ///
    /// Since every valid FieldName is a valid FieldValue this is done infallibly.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::field::{FieldValue, FieldName};
    /// # use http::field::ACCEPT;
    /// let val = FieldValue::from_name(ACCEPT);
    /// assert_eq!(val, FieldValue::from_bytes(b"accept").unwrap());
    /// ```
    #[inline]
    pub fn from_name(name: FieldName) -> FieldValue {
        name.into()
    }

    /// Attempt to convert a byte slice to a `FieldValue`.
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
    /// # use http::field::FieldValue;
    /// let val = FieldValue::from_bytes(b"hello\xfa").unwrap();
    /// assert_eq!(val, &b"hello\xfa"[..]);
    /// ```
    ///
    /// An invalid value
    ///
    /// ```
    /// # use http::field::FieldValue;
    /// let val = FieldValue::from_bytes(b"\n");
    /// assert!(val.is_err());
    /// ```
    #[inline]
    pub fn from_bytes(src: &[u8]) -> Result<FieldValue, InvalidFieldValue> {
        FieldValue::try_from_generic(src, Bytes::copy_from_slice)
    }

    /// Attempt to convert a `Bytes` buffer to a `FieldValue`.
    ///
    /// This will try to prevent a copy if the type passed is the type used
    /// internally, and will copy the data if it is not.
    pub fn from_maybe_shared<T>(src: T) -> Result<FieldValue, InvalidFieldValue>
    where
        T: AsRef<[u8]> + 'static,
    {
        if_downcast_into!(T, Bytes, src, {
            return FieldValue::from_shared(src);
        });

        FieldValue::from_bytes(src.as_ref())
    }

    /// Convert a `Bytes` directly into a `FieldValue` without validating.
    ///
    /// This function does NOT validate that illegal bytes are not contained
    /// within the buffer.
    pub unsafe fn from_maybe_shared_unchecked<T>(src: T) -> FieldValue
    where
        T: AsRef<[u8]> + 'static,
    {
        if cfg!(debug_assertions) {
            match FieldValue::from_maybe_shared(src) {
                Ok(val) => val,
                Err(_err) => {
                    panic!("FieldValue::from_maybe_shared_unchecked() with invalid bytes");
                }
            }
        } else {

            if_downcast_into!(T, Bytes, src, {
                return FieldValue {
                    inner: src,
                    is_sensitive: false,
                };
            });

            let src = Bytes::copy_from_slice(src.as_ref());
            FieldValue {
                inner: src,
                is_sensitive: false,
            }
        }
    }

    fn from_shared(src: Bytes) -> Result<FieldValue, InvalidFieldValue> {
        FieldValue::try_from_generic(src, std::convert::identity)
    }

    fn try_from_generic<T: AsRef<[u8]>, F: FnOnce(T) -> Bytes>(src: T, into: F) -> Result<FieldValue, InvalidFieldValue> {
        for &b in src.as_ref() {
            if !is_valid(b) {
                return Err(InvalidFieldValue { _priv: () });
            }
        }
        Ok(FieldValue {
            inner: into(src),
            is_sensitive: false,
        })
    }

    /// Yields a `&str` slice if the `FieldValue` only contains visible ASCII
    /// chars.
    ///
    /// This function will perform a scan of the header value, checking all the
    /// characters.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::field::FieldValue;
    /// let val = FieldValue::from_static("hello");
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
    /// # use http::field::FieldValue;
    /// let val = FieldValue::from_static("hello");
    /// assert_eq!(val.len(), 5);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.as_ref().len()
    }

    /// Returns true if the `FieldValue` has a length of zero bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::field::FieldValue;
    /// let val = FieldValue::from_static("");
    /// assert!(val.is_empty());
    ///
    /// let val = FieldValue::from_static("hello");
    /// assert!(!val.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Converts a `FieldValue` to a byte slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::field::FieldValue;
    /// let val = FieldValue::from_static("hello");
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
    /// # use http::field::FieldValue;
    /// let mut val = FieldValue::from_static("my secret");
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
    /// be stored on disk or in memory. By marking header values as sensitive,
    /// components using this crate can be instructed to treat them with special
    /// care for security reasons. For example, caches can avoid storing
    /// sensitive values, and HPACK encoders used by HTTP/2.0 implementations
    /// can choose not to compress them.
    ///
    /// Additionally, sensitive values will be masked by the `Debug`
    /// implementation of `FieldValue`.
    ///
    /// Note that sensitivity is not factored into equality or ordering.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::field::FieldValue;
    /// let mut val = FieldValue::from_static("my secret");
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

impl AsRef<[u8]> for FieldValue {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_sensitive {
            f.write_str("Sensitive")
        } else {
            f.write_str("\"")?;
            let mut from = 0;
            let bytes = self.as_bytes();
            for (i, &b) in bytes.iter().enumerate() {
                if !is_visible_ascii(b) || b == b'"' {
                    if from != i {
                        f.write_str(unsafe { str::from_utf8_unchecked(&bytes[from..i]) })?;
                    }
                    if b == b'"' {
                        f.write_str("\\\"")?;
                    } else {
                        write!(f, "\\x{:x}", b)?;
                    }
                    from = i + 1;
                }
            }

            f.write_str(unsafe { str::from_utf8_unchecked(&bytes[from..]) })?;
            f.write_str("\"")
        }
    }
}

impl From<FieldName> for FieldValue {
    #[inline]
    fn from(h: FieldName) -> FieldValue {
        FieldValue {
            inner: h.into_bytes(),
            is_sensitive: false,
        }
    }
}

macro_rules! from_integers {
    ($($name:ident: $t:ident => $max_len:expr),*) => {$(
        impl From<$t> for FieldValue {
            fn from(num: $t) -> FieldValue {
                let mut buf = if mem::size_of::<BytesMut>() - 1 < $max_len {
                    // On 32bit platforms, BytesMut max inline size
                    // is 15 bytes, but the $max_len could be bigger.
                    //
                    // The likelihood of the number *actually* being
                    // that big is very small, so only allocate
                    // if the number needs that space.
                    //
                    // The largest decimal number in 15 digits:
                    // It wold be 10.pow(15) - 1, but this is a constant
                    // version.
                    if num as u64 > 999_999_999_999_999_999 {
                        BytesMut::with_capacity($max_len)
                    } else {
                        // fits inline...
                        BytesMut::new()
                    }
                } else {
                    // full value fits inline, so don't allocate!
                    BytesMut::new()
                };
                let _ = buf.write_str(::itoa::Buffer::new().format(num));
                FieldValue {
                    inner: buf.freeze(),
                    is_sensitive: false,
                }
            }
        }

        #[test]
        fn $name() {
            let n: $t = 55;
            let val = FieldValue::from(n);
            assert_eq!(val, &n.to_string());

            let n = ::std::$t::MAX;
            let val = FieldValue::from(n);
            assert_eq!(val, &n.to_string());
        }
    )*};
}

from_integers! {
    // integer type => maximum decimal length

    // u8 purposely left off... FieldValue::from(b'3') could be confusing
    from_u16: u16 => 5,
    from_i16: i16 => 6,
    from_u32: u32 => 10,
    from_i32: i32 => 11,
    from_u64: u64 => 20,
    from_i64: i64 => 20
}

#[cfg(target_pointer_width = "16")]
from_integers! {
    from_usize: usize => 5,
    from_isize: isize => 6
}

#[cfg(target_pointer_width = "32")]
from_integers! {
    from_usize: usize => 10,
    from_isize: isize => 11
}

#[cfg(target_pointer_width = "64")]
from_integers! {
    from_usize: usize => 20,
    from_isize: isize => 20
}

#[cfg(test)]
mod from_header_name_tests {
    use super::*;
    use crate::field::map::FieldMap;
    use crate::field::name;

    #[test]
    fn it_can_insert_header_name_as_header_value() {
        let mut map = FieldMap::new();
        map.insert(name::UPGRADE, name::SEC_WEBSOCKET_PROTOCOL.into());
        map.insert(
            name::ACCEPT,
            name::FieldName::from_bytes(b"hello-world").unwrap().into(),
        );

        assert_eq!(
            map.get(name::UPGRADE).unwrap(),
            FieldValue::from_bytes(b"sec-websocket-protocol").unwrap()
        );

        assert_eq!(
            map.get(name::ACCEPT).unwrap(),
            FieldValue::from_bytes(b"hello-world").unwrap()
        );
    }
}

impl FromStr for FieldValue {
    type Err = InvalidFieldValue;

    #[inline]
    fn from_str(s: &str) -> Result<FieldValue, Self::Err> {
        FieldValue::from_str(s)
    }
}

impl<'a> From<&'a FieldValue> for FieldValue {
    #[inline]
    fn from(t: &'a FieldValue) -> Self {
        t.clone()
    }
}

impl<'a> TryFrom<&'a str> for FieldValue {
    type Error = InvalidFieldValue;

    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        t.parse()
    }
}

impl<'a> TryFrom<&'a String> for FieldValue {
    type Error = InvalidFieldValue;
    #[inline]
    fn try_from(s: &'a String) -> Result<Self, Self::Error> {
        Self::from_bytes(s.as_bytes())
    }
}

impl<'a> TryFrom<&'a [u8]> for FieldValue {
    type Error = InvalidFieldValue;

    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        FieldValue::from_bytes(t)
    }
}

impl TryFrom<String> for FieldValue {
    type Error = InvalidFieldValue;

    #[inline]
    fn try_from(t: String) -> Result<Self, Self::Error> {
        FieldValue::from_shared(t.into())
    }
}

impl TryFrom<Vec<u8>> for FieldValue {
    type Error = InvalidFieldValue;

    #[inline]
    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
        FieldValue::from_shared(vec.into())
    }
}

#[cfg(test)]
mod try_from_header_name_tests {
    use super::*;
    use crate::field::name;

    #[test]
    fn it_converts_using_try_from() {
        assert_eq!(
            FieldValue::try_from(name::UPGRADE).unwrap(),
            FieldValue::from_bytes(b"upgrade").unwrap()
        );
    }
}

const fn is_visible_ascii(b: u8) -> bool {
    b >= 32 && b < 127 || b == b'\t'
}

#[inline]
fn is_valid(b: u8) -> bool {
    b >= 32 && b != 127 || b == b'\t'
}

impl fmt::Debug for InvalidFieldValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InvalidFieldValue")
            // skip _priv noise
            .finish()
    }
}

impl fmt::Display for InvalidFieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("failed to parse header value")
    }
}

impl Error for InvalidFieldValue {}

impl fmt::Display for ToStrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("failed to convert header to a str")
    }
}

impl Error for ToStrError {}

// ===== PartialEq / PartialOrd =====

impl PartialEq for FieldValue {
    #[inline]
    fn eq(&self, other: &FieldValue) -> bool {
        self.inner == other.inner
    }
}

impl Eq for FieldValue {}

impl PartialOrd for FieldValue {
    #[inline]
    fn partial_cmp(&self, other: &FieldValue) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl Ord for FieldValue {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl PartialEq<str> for FieldValue {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.inner == other.as_bytes()
    }
}

impl PartialEq<[u8]> for FieldValue {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.inner == other
    }
}

impl PartialOrd<str> for FieldValue {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<cmp::Ordering> {
        (*self.inner).partial_cmp(other.as_bytes())
    }
}

impl PartialOrd<[u8]> for FieldValue {
    #[inline]
    fn partial_cmp(&self, other: &[u8]) -> Option<cmp::Ordering> {
        (*self.inner).partial_cmp(other)
    }
}

impl PartialEq<FieldValue> for str {
    #[inline]
    fn eq(&self, other: &FieldValue) -> bool {
        *other == *self
    }
}

impl PartialEq<FieldValue> for [u8] {
    #[inline]
    fn eq(&self, other: &FieldValue) -> bool {
        *other == *self
    }
}

impl PartialOrd<FieldValue> for str {
    #[inline]
    fn partial_cmp(&self, other: &FieldValue) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

impl PartialOrd<FieldValue> for [u8] {
    #[inline]
    fn partial_cmp(&self, other: &FieldValue) -> Option<cmp::Ordering> {
        self.partial_cmp(other.as_bytes())
    }
}

impl PartialEq<String> for FieldValue {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        *self == &other[..]
    }
}

impl PartialOrd<String> for FieldValue {
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(other.as_bytes())
    }
}

impl PartialEq<FieldValue> for String {
    #[inline]
    fn eq(&self, other: &FieldValue) -> bool {
        *other == *self
    }
}

impl PartialOrd<FieldValue> for String {
    #[inline]
    fn partial_cmp(&self, other: &FieldValue) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

impl<'a> PartialEq<FieldValue> for &'a FieldValue {
    #[inline]
    fn eq(&self, other: &FieldValue) -> bool {
        **self == *other
    }
}

impl<'a> PartialOrd<FieldValue> for &'a FieldValue {
    #[inline]
    fn partial_cmp(&self, other: &FieldValue) -> Option<cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

impl<'a, T: ?Sized> PartialEq<&'a T> for FieldValue
where
    FieldValue: PartialEq<T>,
{
    #[inline]
    fn eq(&self, other: &&'a T) -> bool {
        *self == **other
    }
}

impl<'a, T: ?Sized> PartialOrd<&'a T> for FieldValue
where
    FieldValue: PartialOrd<T>,
{
    #[inline]
    fn partial_cmp(&self, other: &&'a T) -> Option<cmp::Ordering> {
        self.partial_cmp(*other)
    }
}

impl<'a> PartialEq<FieldValue> for &'a str {
    #[inline]
    fn eq(&self, other: &FieldValue) -> bool {
        *other == *self
    }
}

impl<'a> PartialOrd<FieldValue> for &'a str {
    #[inline]
    fn partial_cmp(&self, other: &FieldValue) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

#[test]
fn test_try_from() {
    FieldValue::try_from(vec![127]).unwrap_err();
}

#[test]
fn test_debug() {
    let cases = &[
        ("hello", "\"hello\""),
        ("hello \"world\"", "\"hello \\\"world\\\"\""),
        ("\u{7FFF}hello", "\"\\xe7\\xbf\\xbfhello\""),
    ];

    for &(value, expected) in cases {
        let val = FieldValue::from_bytes(value.as_bytes()).unwrap();
        let actual = format!("{:?}", val);
        assert_eq!(expected, actual);
    }

    let mut sensitive = FieldValue::from_static("password");
    sensitive.set_sensitive(true);
    assert_eq!("Sensitive", format!("{:?}", sensitive));
}
