use byte_str::ByteStr;

use std::fmt;

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

impl fmt::Debug for HeaderValue {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), fmt)
    }
}
