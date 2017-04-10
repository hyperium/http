//! A fast hash function implementation
//!
//! Note that the focus of this module is on **fast** at the expense of a
//! potentially "OK" distribution of hash codes. The goal is not to be resilient
//! to any attack or even header sets that might have high collision rates. The
//! worst case scenario is a sequential search, which, in practice, is actually
//! not terrible. In a DOS attack, `HeaderMap` will switch to a safe hash
//! function.

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

/// Return a hash code for the input buffer
#[inline]
pub fn fast_hash(buf: &[u8]) -> u64 {
    let mut hash = HASH_INIT;
    let mut mult = MULT_INIT;

    unsafe {
        let mut ptr = buf.as_ptr() as *const u8;
        let end_ptr = buf.as_ptr().offset(buf.len() as isize & ROUND_TO_8);

        while end_ptr > ptr {
            let curr = *(ptr as *const u64);

            hash = hash.wrapping_add(curr).wrapping_mul(mult);
            mult = (mult << 5).wrapping_sub(mult);

            ptr = ptr.offset(8);
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

        let val = unsafe { *(buf.as_ptr() as *const u64) };

        self.hash = self.hash.wrapping_add(val).wrapping_mul(self.mult);
        self.mult = (self.mult << 5).wrapping_sub(self.mult);
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
        1 => {
            let curr = *(ptr as *const u8);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);
        }
        2 => {
            let curr = *(ptr as *const u16);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);
        }
        3 => {
            let curr = *(ptr as *const u16);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);
            mult = (mult << 5).wrapping_sub(mult);

            ptr = ptr.offset(2);

            let curr = *(ptr as *const u8);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);
        }
        4 => {
            let curr = *(ptr as *const u32);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);
        }
        5 => {
            let curr = *(ptr as *const u32);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);

            ptr = ptr.offset(4);

            let curr = *(ptr as *const u8);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);
        }
        6 => {
            let curr = *(ptr as *const u32);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);

            ptr = ptr.offset(4);

            let curr = *(ptr as *const u16);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);
        }
        7 => {
            let curr = *(ptr as *const u32);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);

            ptr = ptr.offset(4);

            let curr = *(ptr as *const u16);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);
            mult = (mult << 5).wrapping_sub(mult);

            ptr = ptr.offset(2);

            let curr = *(ptr as *const u8);
            hash = hash.wrapping_add(curr as u64).wrapping_mul(mult);
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
