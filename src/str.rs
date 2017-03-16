use bytes::Bytes;

use std::{ops, str};

// TODO: Move this into `bytes`
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Str {
    bytes: Bytes,
}

impl Str {
    /*
    pub fn new() -> Str {
        Str { bytes: Bytes::new() }
    }

    pub fn from_static(val: &'static str) -> Str {
        Str { bytes: Bytes::from_static(val.as_bytes()) }
    }

    pub fn from_utf8(bytes: Bytes) -> Result<Str, FromUtf8Error> {
        if let Err(e) = str::from_utf8(&bytes[..]) {
            return Err(FromUtf8Error {
                err: e,
                val: bytes,
            });
        }

        Ok(Str { bytes: bytes })
    }

    pub unsafe fn from_utf8_unchecked(bytes: Bytes) -> Str {
        Str { bytes: bytes }
    }
    */
}

impl From<String> for Str {
    fn from(src: String) -> Str {
        Str { bytes: Bytes::from(src) }
    }
}

impl<'a> From<&'a str> for Str {
    fn from(src: &'a str) -> Str {
        Str { bytes: Bytes::from(src) }
    }
}

impl ops::Deref for Str {
    type Target = str;

    fn deref(&self) -> &str {
        let b: &[u8] = self.bytes.as_ref();
        unsafe { str::from_utf8_unchecked(b) }
    }
}
