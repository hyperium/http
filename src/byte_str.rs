use bytes::Bytes;

use std::{ops, str};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct ByteStr {
    bytes: Bytes,
}

impl ByteStr {
    pub fn from_static(val: &'static str) -> ByteStr {
        ByteStr { bytes: Bytes::from_static(val.as_bytes()) }
    }
}

impl ops::Deref for ByteStr {
    type Target = str;

    fn deref(&self) -> &str {
        let b: &[u8] = self.bytes.as_ref();
        unsafe { str::from_utf8_unchecked(b) }
    }
}

impl From<String> for ByteStr {
    fn from(src: String) -> ByteStr {
        ByteStr { bytes: Bytes::from(src) }
    }
}

impl<'a> From<&'a str> for ByteStr {
    fn from(src: &'a str) -> ByteStr {
        ByteStr { bytes: Bytes::from(src) }
    }
}
