use byte_str::ByteStr;

use std::{cmp, fmt};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct HeaderValue {
    inner: ByteStr,
}

impl HeaderValue {
    pub fn as_str(&self) -> &str {
        self.inner.as_ref()
    }

    pub fn len(&self) -> usize {
        self.as_str().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl AsRef<str> for HeaderValue {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for HeaderValue {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl From<String> for HeaderValue {
    fn from(src: String) -> HeaderValue {
        HeaderValue { inner: src.into() }
    }
}

impl<'a> From<&'a str> for HeaderValue {
    fn from(src: &'a str) -> HeaderValue {
        HeaderValue { inner: src.into() }
    }
}

impl fmt::Debug for HeaderValue {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), fmt)
    }
}

// ===== PartialEq / PartialOrd =====

impl PartialEq<str> for HeaderValue {
    fn eq(&self, other: &str) -> bool {
        &*self.inner == other
    }
}

impl PartialOrd<str> for HeaderValue {
    fn partial_cmp(&self, other: &str) -> Option<cmp::Ordering> {
        (*self.inner).partial_cmp(other)
    }
}

impl PartialEq<HeaderValue> for str {
    fn eq(&self, other: &HeaderValue) -> bool {
        *other == *self
    }
}

impl PartialOrd<HeaderValue> for str {
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        other.partial_cmp(self)
    }
}

impl PartialEq<String> for HeaderValue {
    fn eq(&self, other: &String) -> bool {
        *self == &other[..]
    }
}

impl PartialOrd<String> for HeaderValue {
    fn partial_cmp(&self, other: &String) -> Option<cmp::Ordering> {
        (*self.inner).partial_cmp(&other[..])
    }
}

impl PartialEq<HeaderValue> for String {
    fn eq(&self, other: &HeaderValue) -> bool {
        *other == *self
    }
}

impl PartialOrd<HeaderValue> for String {
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        other.partial_cmp(self)
    }
}

impl<'a, T: ?Sized> PartialEq<&'a T> for HeaderValue
    where HeaderValue: PartialEq<T>
{
    fn eq(&self, other: &&'a T) -> bool {
        *self == **other
    }
}

impl<'a, T: ?Sized> PartialOrd<&'a T> for HeaderValue
    where HeaderValue: PartialOrd<T>
{
    fn partial_cmp(&self, other: &&'a T) -> Option<cmp::Ordering> {
        self.partial_cmp(*other)
    }
}

impl<'a> PartialEq<HeaderValue> for &'a str {
    fn eq(&self, other: &HeaderValue) -> bool {
        *other == *self
    }
}

impl<'a> PartialOrd<HeaderValue> for &'a str {
    fn partial_cmp(&self, other: &HeaderValue) -> Option<cmp::Ordering> {
        other.partial_cmp(self)
    }
}
