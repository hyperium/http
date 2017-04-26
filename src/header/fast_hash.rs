//! A fast hash function implementation
//!
//! Note that the focus of this module is on **fast** at the expense of a
//! potentially "OK" distribution of hash codes. The goal is not to be resilient
//! to any attack or even header sets that might have high collision rates. The
//! worst case scenario is a sequential search, which, in practice, is actually
//! not terrible. In a DOS attack, `HeaderMap` will switch to a safe hash
//! function.

use std::{mem, ptr};
use std::hash::Hash;

/// A fast hashable type
///
/// Implementations of `FastHash` provie an optimized hash function that offers
/// no security guarantees and an accaptable distribution of hash values for
/// simple cases.
pub trait FastHash: Hash {
    fn fast_hash(&self) -> u64;
}

/// Hashes an input stream in chunk of 8 bytes.
pub struct FastHasher {
    hash: u64,
    mult: u64,
}

const HASH_INIT: u64 = 0;
const MULT_INIT: u64 = 1;
const ROUND_TO_8: isize = !7;

macro_rules! hash_num {
    ($hash:expr, $mult:expr, $num:expr) => {
        $hash = $hash.wrapping_add($num as u64).wrapping_mul($mult);
    };
}

macro_rules! diffuse {
    ($mult:expr) => {
        $mult = ($mult << 5).wrapping_sub($mult);
    }
}

macro_rules! hash_final_chunk {
    ($hash:ident, $mult:ident, $ptr:ident, $ty:ty) => {{
        hash_num!($hash, $mult, ptr::read_unaligned($ptr as *const $ty));
    }};
}

macro_rules! hash_chunk {
    ($hash:ident, $mult:ident, $ptr:ident, $ty:ty) => {{
        hash_final_chunk!($hash, $mult, $ptr, $ty);

        diffuse!($mult);
        $ptr = $ptr.offset(mem::size_of::<$ty>() as isize);
    }};
}

/// Return a hash code for the input buffer
#[inline]
pub fn fast_hash(buf: &[u8]) -> u64 {
    // This function requires that the size of the given buffer is less than
    // uszie::MAX >> 1. We don't check for this in the function, but `fast_hash`
    // is a private function and is only used with header names, which are
    // limited to 64kb.

    let mut hash = HASH_INIT;
    let mut mult = MULT_INIT;

    unsafe {
        let mut ptr = buf.as_ptr() as *const u8;
        let end_ptr = buf.as_ptr().offset(buf.len() as isize & ROUND_TO_8);

        while end_ptr > ptr {
            hash_chunk!(hash, mult, ptr, u64);
        }

        finish(ptr, buf.len() & 7, hash, mult)
    }
}

impl FastHasher {
    pub fn new() -> FastHasher {
        FastHasher {
            hash: HASH_INIT,
            mult: MULT_INIT,
        }
    }

    pub fn hash(&mut self, buf: &[u8]) {
        assert_eq!(8, buf.len());

        let num = unsafe { ptr::read_unaligned(buf.as_ptr() as *const u64) };

        hash_num!(self.hash, self.mult, num);
        diffuse!(self.mult);
    }

    pub fn finish(&mut self, buf: &[u8]) -> u64{
        assert!(buf.len() < 8);
        unsafe {
            finish(
                buf.as_ptr(),
                buf.len(),
                self.hash,
                self.mult)
        }
    }
}

#[inline]
unsafe fn finish(mut ptr: *const u8, rem: usize, mut hash: u64, mut mult: u64) -> u64 {
    match rem {
        0 => {}
        1 => hash_final_chunk!(hash, mult, ptr, u8),
        2 => hash_final_chunk!(hash, mult, ptr, u16),
        3 => {
            hash_chunk!(hash, mult,ptr, u16);
            hash_final_chunk!(hash, mult,ptr, u8);
        }
        4 => hash_final_chunk!(hash, mult,ptr, u32),
        5 => {
            hash_chunk!(hash, mult,ptr, u32);
            hash_final_chunk!(hash, mult,ptr, u8);
        }
        6 => {
            hash_chunk!(hash, mult,ptr, u32);
            hash_final_chunk!(hash, mult,ptr, u16);
        }
        7 => {
            hash_chunk!(hash, mult,ptr, u32);
            hash_chunk!(hash, mult,ptr, u16);
            hash_final_chunk!(hash, mult,ptr, u8);
        }
        _ => unreachable!(),
    }

    hash
}

#[test]
fn test_fast_hash() {
    const HEADERS: &'static [&'static str] = &[
        "accept",
        "accept-charset",
        "accept-encoding",
        "accept-language",
        "accept-patch",
        "accept-ranges",
        "access-control-allow-credentials",
        "access-control-allow-headers",
        "access-control-allow-methods",
        "access-control-allow-origin",
        "access-control-expose-headers",
        "access-control-max-age",
        "access-control-request-headers",
        "access-control-request-method",
        "age",
        "allow",
        "alt-svc",
        "authorization",
        "cache-control",
        "connection",
        "content-disposition",
        "content-encoding",
        "content-language",
        "content-length",
        "content-location",
        "content-md5",
        "content-range",
        "content-security-policy",
        "content-security-policy-report-only",
        "content-type",
        "cookie",
        "dnt",
        "date",
        "etag",
        "expect",
        "expires",
        "forwarded",
        "from",
        "host",
        "if-match",
        "if-modified-since",
        "if-none-match",
        "if-range",
        "if-unmodified-since",
        "last-modified",
        "keep-alive",
        "link",
        "location",
        "max-forwards",
        "origin",
        "pragma",
        "proxy-authenticate",
        "proxy-authorization",
        "public-key-pins",
        "public-key-pins-report-only",
        "range",
        "referer",
        "referrer-policy",
        "refresh",
        "retry-after",
        "server",
        "set-cookie",
        "strict-transport-security",
        "te",
        "tk",
        "trailer",
        "transfer-encoding",
        "tsv",
        "user-agent",
        "upgrade",
        "upgrade-insecure-requests",
        "vary",
        "via",
        "warning",
        "www-authenticate",
        "x-content-type-options",
        "x-dns-prefetch-control",
        "x-frame-options",
        "x-xss-protection",
    ];

    for (i, hdr) in HEADERS.iter().enumerate() {
        let len = hdr.len();

        let a = fast_hash(hdr.as_bytes());

        let mut hasher = FastHasher::new();
        let mut buf = hdr.as_bytes();

        while buf.len() >= 8 {
            hasher.hash(&buf[..8]);
            buf = &buf[8..];
        }

        let b = hasher.finish(buf);
        assert_eq!(a, b, "failed hash {:?}", hdr);

        let mut buf = [i as u8; 256];
        buf[0..len].copy_from_slice(hdr.as_bytes());

        let c = fast_hash(&buf[..len]);
        assert_eq!(a, c);

        let mut hasher = FastHasher::new();
        let mut buf = &buf[..len];

        while buf.len() >= 8 {
            hasher.hash(&buf[..8]);
            buf = &buf[8..];
        }

        let d = hasher.finish(buf);
        assert_eq!(a, d, "failed hash {:?}", hdr);
    }
}
