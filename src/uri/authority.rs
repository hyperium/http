use std::convert::TryFrom;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::{cmp, fmt, str};

use bytes::Bytes;

use super::{ErrorKind, InvalidUri, Port, URI_CHARS};
use crate::byte_str::ByteStr;

/// Validation result for authority parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)] // to compare in const contexts
enum AuthorityError {
    Empty,
    InvalidUriChar,
    InvalidAtUsage,
    InvalidBracketUsage,
    InvalidUserinfo,
    InvalidHostname,
    InvalidPercent,
    InvalidPort,
    InvalidIpv6,
    InvalidZoneId,
}

/// Represents the authority component of a URI.
#[derive(Clone)]
pub struct Authority {
    pub(super) data: ByteStr,
}

impl Authority {
    pub(super) fn empty() -> Self {
        Authority {
            data: ByteStr::new(),
        }
    }

    // Not public while `bytes` is unstable.
    pub(super) fn from_shared(s: Bytes) -> Result<Self, InvalidUri> {
        // Precondition on create_authority: trivially satisfied by the
        // identity closure
        create_authority(s, |s| s)
    }

    /// Attempt to convert an `Authority` from a static string.
    ///
    /// This function will not perform any copying, and the string will be
    /// checked if it is empty or contains an invalid character.
    ///
    /// # Panics
    ///
    /// This function panics if the argument contains invalid characters or
    /// is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority = Authority::from_static("example.com");
    /// assert_eq!(authority.host(), "example.com");
    /// ```
    #[inline]
    pub const fn from_static(src: &'static str) -> Self {
        match validate_authority_bytes(src.as_bytes()) {
            Ok(_) => Authority {
                data: ByteStr::from_static(src),
            },
            Err(_) => panic!("static str is not valid authority"),
        }
    }

    /// Attempt to convert a `Bytes` buffer to a `Authority`.
    ///
    /// This will try to prevent a copy if the type passed is the type used
    /// internally, and will copy the data if it is not.
    pub fn from_maybe_shared<T>(src: T) -> Result<Self, InvalidUri>
    where
        T: AsRef<[u8]> + 'static,
    {
        if_downcast_into!(T, Bytes, src, {
            return Authority::from_shared(src);
        });

        Authority::try_from(src.as_ref())
    }

    // Note: this may return an *empty* Authority. You might want `parse_non_empty`.
    // Postcondition: for all Ok() returns, s[..ret.unwrap()] is valid UTF-8 where
    // ret is the return value.
    pub(super) fn parse(s: &[u8]) -> Result<usize, InvalidUri> {
        validate_authority_bytes(s).map_err(|e| {
            match e {
                AuthorityError::Empty => ErrorKind::Empty,
                AuthorityError::InvalidUriChar => ErrorKind::InvalidUriChar,
                AuthorityError::InvalidPort => ErrorKind::InvalidPort,
                _ => ErrorKind::InvalidAuthority,
            }
            .into()
        })
    }

    // Parse bytes as an Authority, not allowing an empty string.
    //
    // This should be used by functions that allow a user to parse
    // an `Authority` by itself.
    //
    // Postcondition: for all Ok() returns, s[..ret.unwrap()] is valid UTF-8 where
    // ret is the return value.
    fn parse_non_empty(s: &[u8]) -> Result<usize, InvalidUri> {
        if s.is_empty() {
            return Err(ErrorKind::Empty.into());
        }
        Authority::parse(s)
    }

    /// Get the host of this `Authority`.
    ///
    /// The host subcomponent of authority is identified by an IP literal
    /// encapsulated within square brackets, an IPv4 address in dotted- decimal
    /// form, or a registered name.  The host subcomponent is **case-insensitive**.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                         |---------|
    ///                              |
    ///                             host
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::uri::*;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// assert_eq!(authority.host(), "example.org");
    /// ```
    #[inline]
    pub fn host(&self) -> &str {
        host(self.as_str())
    }

    /// Get the port part of this `Authority`.
    ///
    /// The port subcomponent of authority is designated by an optional port
    /// number following the host and delimited from it by a single colon (":")
    /// character. It can be turned into a decimal port number with the `as_u16`
    /// method or as a `str` with the `as_str` method.
    ///
    /// ```notrust
    /// abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1
    ///                                     |-|
    ///                                      |
    ///                                     port
    /// ```
    ///
    /// # Examples
    ///
    /// Authority with port
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// let port = authority.port().unwrap();
    /// assert_eq!(port.as_u16(), 80);
    /// assert_eq!(port.as_str(), "80");
    /// ```
    ///
    /// Authority without port
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org".parse().unwrap();
    ///
    /// assert!(authority.port().is_none());
    /// ```
    pub fn port(&self) -> Option<Port<&str>> {
        let bytes = self.as_str();
        bytes
            .rfind(':')
            .and_then(|i| Port::from_str(&bytes[i + 1..]).ok())
    }

    /// Get the port of this `Authority` as a `u16`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::uri::Authority;
    /// let authority: Authority = "example.org:80".parse().unwrap();
    ///
    /// assert_eq!(authority.port_u16(), Some(80));
    /// ```
    pub fn port_u16(&self) -> Option<u16> {
        self.port().map(|p| p.as_u16())
    }

    /// Return a str representation of the authority
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.data[..]
    }
}

// Purposefully not public while `bytes` is unstable.
// impl TryFrom<Bytes> for Authority

impl AsRef<str> for Authority {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for Authority {
    fn eq(&self, other: &Authority) -> bool {
        self.data.eq_ignore_ascii_case(&other.data)
    }
}

impl Eq for Authority {}

/// Case-insensitive equality
///
/// # Examples
///
/// ```
/// # use http::uri::Authority;
/// let authority: Authority = "HELLO.com".parse().unwrap();
/// assert_eq!(authority, "hello.coM");
/// assert_eq!("hello.com", authority);
/// ```
impl PartialEq<str> for Authority {
    fn eq(&self, other: &str) -> bool {
        self.data.eq_ignore_ascii_case(other)
    }
}

impl PartialEq<Authority> for str {
    fn eq(&self, other: &Authority) -> bool {
        self.eq_ignore_ascii_case(other.as_str())
    }
}

impl PartialEq<Authority> for &str {
    fn eq(&self, other: &Authority) -> bool {
        self.eq_ignore_ascii_case(other.as_str())
    }
}

impl PartialEq<&str> for Authority {
    fn eq(&self, other: &&str) -> bool {
        self.data.eq_ignore_ascii_case(other)
    }
}

impl PartialEq<String> for Authority {
    fn eq(&self, other: &String) -> bool {
        self.data.eq_ignore_ascii_case(other.as_str())
    }
}

impl PartialEq<Authority> for String {
    fn eq(&self, other: &Authority) -> bool {
        self.as_str().eq_ignore_ascii_case(other.as_str())
    }
}

/// Case-insensitive ordering
///
/// # Examples
///
/// ```
/// # use http::uri::Authority;
/// let authority: Authority = "DEF.com".parse().unwrap();
/// assert!(authority < "ghi.com");
/// assert!(authority > "abc.com");
/// ```
impl PartialOrd for Authority {
    fn partial_cmp(&self, other: &Authority) -> Option<cmp::Ordering> {
        let left = self.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}

impl PartialOrd<str> for Authority {
    fn partial_cmp(&self, other: &str) -> Option<cmp::Ordering> {
        let left = self.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}

impl PartialOrd<Authority> for str {
    fn partial_cmp(&self, other: &Authority) -> Option<cmp::Ordering> {
        let left = self.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}

impl PartialOrd<Authority> for &str {
    fn partial_cmp(&self, other: &Authority) -> Option<cmp::Ordering> {
        let left = self.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}

impl PartialOrd<&str> for Authority {
    fn partial_cmp(&self, other: &&str) -> Option<cmp::Ordering> {
        let left = self.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}

impl PartialOrd<String> for Authority {
    fn partial_cmp(&self, other: &String) -> Option<cmp::Ordering> {
        let left = self.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}

impl PartialOrd<Authority> for String {
    fn partial_cmp(&self, other: &Authority) -> Option<cmp::Ordering> {
        let left = self.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        let right = other.data.as_bytes().iter().map(|b| b.to_ascii_lowercase());
        left.partial_cmp(right)
    }
}

/// Case-insensitive hashing
///
/// # Examples
///
/// ```
/// # use http::uri::Authority;
/// # use std::hash::{Hash, Hasher};
/// # use std::collections::hash_map::DefaultHasher;
///
/// let a: Authority = "HELLO.com".parse().unwrap();
/// let b: Authority = "hello.coM".parse().unwrap();
///
/// let mut s = DefaultHasher::new();
/// a.hash(&mut s);
/// let a = s.finish();
///
/// let mut s = DefaultHasher::new();
/// b.hash(&mut s);
/// let b = s.finish();
///
/// assert_eq!(a, b);
/// ```
impl Hash for Authority {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.data.len().hash(state);
        for &b in self.data.as_bytes() {
            state.write_u8(b.to_ascii_lowercase());
        }
    }
}

impl TryFrom<&[u8]> for Authority {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        // parse first, and only turn into Bytes if valid

        // Preconditon on create_authority: copy_from_slice() copies all of
        // bytes from the [u8] parameter into a new Bytes
        create_authority(s, Bytes::copy_from_slice)
    }
}

impl TryFrom<&str> for Authority {
    type Error = InvalidUri;
    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        TryFrom::try_from(s.as_bytes())
    }
}

impl TryFrom<Vec<u8>> for Authority {
    type Error = InvalidUri;

    #[inline]
    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
        Authority::from_shared(vec.into())
    }
}

impl TryFrom<String> for Authority {
    type Error = InvalidUri;

    #[inline]
    fn try_from(t: String) -> Result<Self, Self::Error> {
        Authority::from_shared(t.into())
    }
}

impl FromStr for Authority {
    type Err = InvalidUri;

    fn from_str(s: &str) -> Result<Self, InvalidUri> {
        TryFrom::try_from(s)
    }
}

impl fmt::Debug for Authority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Authority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn host(auth: &str) -> &str {
    let host_port = auth
        .rsplit('@')
        .next()
        .expect("split always has at least 1 item");

    if host_port.as_bytes()[0] == b'[' {
        let i = host_port
            .find(']')
            .expect("parsing should validate brackets");
        // ..= ranges aren't available in 1.20, our minimum Rust version...
        &host_port[0..i + 1]
    } else {
        host_port
            .split(':')
            .next()
            .expect("split always has at least 1 item")
    }
}

// Precondition: f converts all of the bytes in the passed in B into the
// returned Bytes.
fn create_authority<B, F>(b: B, f: F) -> Result<Authority, InvalidUri>
where
    B: AsRef<[u8]>,
    F: FnOnce(B) -> Bytes,
{
    let s = b.as_ref();
    let authority_end = Authority::parse_non_empty(s)?;

    if authority_end != s.len() {
        return Err(ErrorKind::InvalidUriChar.into());
    }

    let bytes = f(b);

    Ok(Authority {
        // Safety: the postcondition on parse_non_empty() and the check against
        // s.len() ensure that b is valid UTF-8. The precondition on f ensures
        // that this is carried through to bytes.
        data: unsafe { ByteStr::from_utf8_unchecked(bytes) },
    })
}

macro_rules! const_try {
    ($e:expr) => {
        match $e {
            Ok(ok) => ok,
            Err(err) => return Err(err),
        }
    };
}

/// unreserved characters according to RFC 3986, section 2.3
#[inline]
const fn is_unreserved(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_' | b'~')
}

/// sub-delim characters according to RFC 3986, section 2.3
#[inline]
const fn is_sub_delims(c: u8) -> bool {
    matches!(
        c,
        b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
    )
}

/// Decode a hex digit into its nibble value (0-15).
#[inline]
const fn hex_decode_nibble(c: u8) -> Result<u8, AuthorityError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        _ => Err(AuthorityError::InvalidPercent),
    }
}

/// Decode a hex byte in at position `i` in a byte string.
#[inline]
const fn hex_decode_byte(s: &[u8], i: usize) -> Result<u8, AuthorityError> {
    if i + 1 >= s.len() {
        return Err(AuthorityError::InvalidPercent);
    };
    Ok(const_try!(hex_decode_nibble(s[i])) << 4 | const_try!(hex_decode_nibble(s[i + 1])))
}

/// Check that the userinfo in s[..i_end] is compliant with RFC 3986, section 2.3.1.
#[inline]
const fn validate_userinfo(s: &[u8], i_end: usize) -> Result<(), AuthorityError> {
    if i_end == 0 {
        return Err(AuthorityError::InvalidUserinfo);
    }

    let mut i = 0;
    while i < i_end {
        let c = s[i];
        if is_unreserved(c) || is_sub_delims(c) || c == b':' {
            i += 1;
        } else if c == b'%' {
            const_try!(hex_decode_byte(s, i + 1));
            i += 3;
        } else {
            return Err(AuthorityError::InvalidUserinfo);
        }
    }
    Ok(())
}

/// Validate a hostname against RFC 3986.
#[inline]
const fn validate_hostname(s: &[u8], i_start: usize, i_end: usize) -> Result<(), AuthorityError> {
    // This check is not part of RFC 3986 (which allows an empty hostname), but
    // RFC 9110 (HTTP Semantics), section 4.2.1, clearly states that empty host
    // identifiers *MUST* be rejected.
    //
    // It also matches the past behavior of the crate, which is intentional
    // (see https://github.com/hyperium/http/pull/698).
    if i_start == i_end {
        return Err(AuthorityError::InvalidHostname);
    }

    let mut i = i_start;
    while i < i_end {
        let c = s[i];
        if is_unreserved(c) || is_sub_delims(c) {
            i += 1
        } else if c == b'%' {
            let encoded = const_try!(hex_decode_byte(s, i + 1));
            // Only allow percent encoded bytes that can't be included as-is.
            //
            // RFC 3986 section 3.2.2 states that URI *producers* must not use percent encoding
            // unless it is used to represent non-ASCII UTF8 sequences. However it does not state
            // that *parsers* must reject ASCII or non-UTF8 percent encodings.
            //
            // To avoid ambiguity with this crate's previous rejection of percent-encoded hosts while
            // allowing e.g. percent-encoded socket paths, we only reject percent encodings that would
            // otherwise be valid reg-name characters.
            if is_unreserved(encoded) || is_sub_delims(encoded) {
                return Err(AuthorityError::InvalidPercent);
            }
            i += 3;
        } else {
            return Err(AuthorityError::InvalidHostname);
        }
    }
    Ok(())
}

/// Validate a zoneid against RFC 6874.
#[inline]
const fn validate_zoneid(s: &[u8], i_start: usize, i_end: usize) -> Result<(), AuthorityError> {
    // The zoneid must not be empty.
    if i_start == i_end {
        return Err(AuthorityError::InvalidZoneId);
    }

    let mut i = i_start;
    while i < i_end {
        let c = s[i];
        if is_unreserved(c) {
            i += 1
        } else if c == b'%' {
            const_try!(hex_decode_byte(s, i + 1));
            i += 3;
        } else {
            return Err(AuthorityError::InvalidZoneId);
        }
    }
    Ok(())
}

/// Parses a mapped IPv4 in an IPv6 address.
///
/// Returns its starting position in the byte slice if successful.
const fn parse_trailing_ipv4(s: &[u8], i_start: usize, i_end: usize) -> Option<usize> {
    let mut octet_cnt = 0;
    let mut digit_mul = 1;
    let mut octet_value = 0;
    let mut first_digit = 0;
    let mut i = i_end;

    while i > i_start {
        i -= 1;
        let c = s[i];
        if (c == b':' && octet_cnt == 3) || (c == b'.' && octet_cnt < 3) {
            // disallow empty fields, trailing zeroes and ensure the octet value is in range
            if first_digit == 0 || (first_digit == b'0' && octet_value > 0) || octet_value >= 256 {
                return None;
            }
            // success after parsing 4 octets
            octet_cnt += 1;
            if octet_cnt == 4 {
                // note: +1 as we don't count the ipv6 separator ':'
                return Some(i + 1);
            }
            // reset octet state
            digit_mul = 1;
            octet_value = 0;
            first_digit = 0;
        } else if digit_mul == 1000 || !c.is_ascii_digit() {
            return None;
        } else {
            first_digit = c;
            octet_value = digit_mul * (c - b'0') as u32 + octet_value;
            digit_mul *= 10;
        }
    }
    None
}

/// Validate an IPv6 address according to RFC 3986.
///
/// Zone identifiers encoded according to RFC 6874 are allowed, despite being
/// obsoleted by RFC 9844, for backwards compatibility.
///
/// This supports mapped IPv4 addresses.
const fn validate_ipv6(s: &[u8], i_start: usize, mut i_end: usize) -> Result<(), AuthorityError> {
    // First find the first instance of '%25' to check if we have a zone identifier,
    // and validate it.
    //
    // We can't use the last '%25', since anything (including another %) may be encoded
    // as part of the zoneid.
    let mut i = i_start;
    while i + 2 < i_end {
        if s[i] == b'%' {
            if const_try!(hex_decode_byte(s, i + 1)) != b'%' {
                return Err(AuthorityError::InvalidIpv6);
            }
            const_try!(validate_zoneid(s, i + 3, i_end));
            i_end = i;
            break;
        }
        i += 1;
    }

    // Then, check if we have a trailing IPv4. If so, there are 6 16-bit groups left.
    // Otherwise there are 8.
    let (mut groups_left, is_mapped) = match parse_trailing_ipv4(s, i_start, i_end) {
        Some(new_end) => {
            i_end = new_end;
            (6, true)
        }
        None => (8, false),
    };

    // The proper ipv6 is at least 2 characters and less than 5*groups_left.
    //
    // Doing this check early allows us to also check if : is in the start position,
    // which the loop below can't do without checking this at each : in the IP.
    let len = i_end - i_start;
    if len < 2 || len > 5 * groups_left || (s[i_start] == b':' && s[i_start + 1] != b':') {
        return Err(AuthorityError::InvalidIpv6);
    }

    let mut has_double_colon = false;
    let mut colon_cnt = 0;
    let mut digit_cnt = 0;
    let mut i = i_start;
    while i < i_end {
        let c = s[i];
        if c.is_ascii_hexdigit() {
            if digit_cnt == 4 {
                return Err(AuthorityError::InvalidIpv6);
            }
            if digit_cnt == 0 {
                if groups_left == 0 {
                    return Err(AuthorityError::InvalidIpv6);
                }
                groups_left -= 1;
            }
            colon_cnt = 0;
            digit_cnt += 1;
        } else if c == b':' {
            if colon_cnt > 0 {
                if has_double_colon {
                    return Err(AuthorityError::InvalidIpv6);
                }
                has_double_colon = true;
            }
            digit_cnt = 0;
            colon_cnt += 1;
        } else {
            return Err(AuthorityError::InvalidIpv6);
        }
        i += 1;
    }

    // we need an ending single colon iff the ip ends with a v4 address.
    if colon_cnt != 2 && (colon_cnt == 1) != is_mapped {
        return Err(AuthorityError::InvalidIpv6);
    }

    // Either we don't have a double colon and specified all groups, or we have
    // one and omitted at least one group.
    if has_double_colon == (groups_left > 0) {
        Ok(())
    } else {
        Err(AuthorityError::InvalidIpv6)
    }
}

/// Validate the port of the authority against RFC 3986, section 3.2.3.
#[inline]
const fn validate_port(s: &[u8], i_start: usize, i_end: usize) -> Result<(), AuthorityError> {
    let mut i = i_start;
    while i < i_end {
        if !s[i].is_ascii_digit() {
            return Err(AuthorityError::InvalidPort);
        }
        i += 1;
    }
    Ok(())
}

/// Validate the authority against RFC 3986, but don't accept empty authorities.
const fn validate_authority_bytes(s: &[u8]) -> Result<usize, AuthorityError> {
    if s.is_empty() {
        return Err(AuthorityError::Empty);
    }

    let mut end = s.len();
    let mut port_colon = None;
    let mut at = None;
    let mut start_bracket = None;
    let mut end_bracket = None;

    let mut i = 0;
    while i < s.len() {
        let b = s[i];
        let ch = URI_CHARS[b as usize];

        if ch == b'/' || ch == b'?' || ch == b'#' {
            end = i;
            break;
        }
        if ch == 0 && b != b'%' {
            return Err(AuthorityError::InvalidUriChar);
        } else if ch == b':' {
            port_colon = Some(i);
        } else if ch == b'@' {
            if at.is_some() || start_bracket.is_some() {
                return Err(AuthorityError::InvalidAtUsage);
            }
            port_colon = None;
            at = Some(i);
        } else if ch == b'[' {
            if start_bracket.is_some() || end_bracket.is_some() {
                return Err(AuthorityError::InvalidBracketUsage);
            }
            start_bracket = Some(i)
        } else if ch == b']' {
            if start_bracket.is_none() || end_bracket.is_some() {
                return Err(AuthorityError::InvalidBracketUsage);
            }
            port_colon = None;
            end_bracket = Some(i)
        }
        i += 1;
    }

    let host_start = match at {
        Some(i) => {
            const_try!(validate_userinfo(s, i));
            i + 1
        }
        None => 0,
    };
    let host_end = match port_colon {
        Some(i) => {
            const_try!(validate_port(s, i + 1, end));
            i
        }
        None => end,
    };

    match (start_bracket, end_bracket) {
        (None, None) => const_try!(validate_hostname(s, host_start, host_end)),
        (Some(start), Some(end)) if start == host_start && end + 1 == host_end => {
            const_try!(validate_ipv6(s, start + 1, end))
        }
        _ => return Err(AuthorityError::InvalidBracketUsage),
    }

    Ok(end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    const fn test_zoneid(s: &[u8], valid: bool) {
        if validate_zoneid(s, 0, s.len()).is_ok() != valid {
            panic!("zoneid test failed")
        }
    }

    const _ZONEID_UNRESERVED: () = test_zoneid(b"wlan0", true);
    const _ZONEID_ANY_PERCENT: () = test_zoneid(b"%00wl%61n%30", true);
    const _ZONEID_EMPTY: () = test_zoneid(b"", false);
    const _ZONEID_BAD_PERCENT1: () = test_zoneid(b"wlan0%1z", false);
    const _ZONEID_BAD_PERCENT2: () = test_zoneid(b"wlan0%%z", false);
    const _ZONEID_SUB_DELIM_INVALID: () = test_zoneid(b"wlan0:", false);
    const _ZONEID_GEN_DELIM_INVALID: () = test_zoneid(b"abc@", false);

    #[allow(dead_code)]
    const fn test_trailing_ipv4(s: &[u8], expected: Option<usize>) {
        match (parse_trailing_ipv4(s, 0, s.len()), expected) {
            (None, None) => (),
            (Some(x), Some(y)) if x == y => (),
            _ => panic!("trailing ipv4 test failed"),
        }
    }

    const _TRAILING_IPV4_OK1: () = test_trailing_ipv4(b":0.0.0.0", Some(1));
    const _TRAILING_IPV4_OK2: () = test_trailing_ipv4(b"2f:127.3.42.53", Some(3));
    const _TRAILING_IPV4_OK3: () = test_trailing_ipv4(b":255.255.255.255", Some(1));
    const _TRAILING_IPV4_TOO_SHORT: () = test_trailing_ipv4(b":0.0.0", None);
    const _TRAILING_IPV4_TOO_LONG: () = test_trailing_ipv4(b":0.0.0.0.0", None);
    const _TRAILING_IPV4_MISSING1: () = test_trailing_ipv4(b":12.0.0.", None);
    const _TRAILING_IPV4_MISSING2: () = test_trailing_ipv4(b":12..0.0", None);
    const _TRAILING_IPV4_MISSING3: () = test_trailing_ipv4(b":..0.0", None);
    const _TRAILING_IPV4_MISSING4: () = test_trailing_ipv4(b":0.0", None);
    const _TRAILING_IPV4_LEADING_ZERO: () = test_trailing_ipv4(b":01.0.0.0", None);
    const _TRAILING_IPV4_TOO_BIG: () = test_trailing_ipv4(b":256.0.0.0", None);

    #[allow(dead_code)]
    const fn test_ipv6(s: &[u8], valid: bool) {
        if validate_ipv6(s, 0, s.len()).is_ok() != valid {
            panic!("ipv6 test failed")
        }
    }

    const _IPV6_FULL: () = test_ipv6(b"a1b2:c34:e5f6:7890:0:c3d4:e5f6:7890", true);
    const _IPV6_FULL_ZONEID: () =
        test_ipv6(b"a1b2:c3d4:e5f6:7890:a1b2:c3d4:e5f6:7890%25eth%30", true);
    const _IPV6_SHORT_START: () = test_ipv6(b"::a1b2", true);
    const _IPV6_SHORT_END: () = test_ipv6(b"cd4:a1b2::", true);
    const _IPV6_SHORT_MIDDLE: () = test_ipv6(b"0:1:234::5:6:7", true);
    const _IPV6_ZERO: () = test_ipv6(b"::", true);
    const _IPV6_MAPPED_FULL_ZONEID: () = test_ipv6(b"a:b:c:d:e:f:123.45.67.9%25eth0", true);
    const _IPV6_MAPPED_SHORT: () = test_ipv6(b"::ffff:123.45.67.8", true);
    const _IPV6_MAPPED_VERY_SHORT: () = test_ipv6(b"::123.45.67.8", true);

    const _IPV6_TOO_SHORT: () = test_ipv6(b"0:1:2:3", false);
    const _IPV6_TOO_LONG: () = test_ipv6(b"0:1:2:3:4:5:6:7:8", false);
    const _IPV6_TOO_LONG_MAPPED: () = test_ipv6(b"0:1:2:3:4:5:6:12.34.56.78", false);
    const _IPV6_TOO_LONG_COMPACT_END: () = test_ipv6(b"0:1:2:3:4:5:6:7::", false);
    const _IPV6_TOO_LONG_COMPACT_MIDDLE: () = test_ipv6(b"0:1:2:3:4::5:6:7", false);
    const _IPV6_TOO_TRAILING_COLON: () = test_ipv6(b"0:1:2:34:5:6:7:", false);
    const _IPV6_TOO_STARTING_COLON: () = test_ipv6(b":0:1:2:34:5:6:7", false);
    const _IPV6_NON_HEX_CHR: () = test_ipv6(b"::45g", false);
    const _IPV6_MORE_THAN_4_CHRS: () = test_ipv6(b"::12020", false);
    const _IPV6_BAD_ZONEID_SEP: () = test_ipv6(b"::%33abc", false);

    #[allow(dead_code)]
    const fn test_hostname(s: &[u8], valid: bool) {
        if validate_hostname(s, 0, s.len()).is_ok() != valid {
            panic!("reg_name test failed")
        }
    }

    const _HOST_NAME: () = test_hostname(b"04-2alx$!~", true);
    const _HOST_NAME_PERCENT: () = test_hostname(b"abc%40%19", true);
    const _HOST_NAME_EMPTY: () = test_hostname(b"", false);
    const _HOST_NAME_COLON: () = test_hostname(b"abc:def", false);
    const _HOST_NAME_RESERVED: () = test_hostname(b"@abc", false);
    const _HOST_NAME_NON_ASCII: () = test_hostname(b"abc\x19", false);
    const _HOST_NAME_BAD_PERCENT: () = test_hostname(b"abc%", false);
    const _HOST_NAME_USELESS_PERCENT1: () = test_hostname(b"abc%24", false); // 0x40 = '$'
    const _HOST_NAME_USELESS_PERCENT2: () = test_hostname(b"abc%5a", false); // 0x5a = 'Z'

    #[allow(dead_code)]
    const fn test_port(s: &[u8], valid: bool) {
        if validate_port(s, 0, s.len()).is_ok() != valid {
            panic!("port test failed")
        }
    }

    const _PORT_OK: () = test_port(b"12345", true);
    const _PORT_EMPTY: () = test_port(b"", true);
    const _PORT_LONG_OK: () = test_port(b"000123456789", true);
    const _PORT_NON_DIGIT: () = test_port(b"a", false);

    #[allow(dead_code)]
    const fn test_userinfo(s: &[u8], valid: bool) {
        if validate_userinfo(s, s.len()).is_ok() != valid {
            panic!("userinfo test failed")
        }
    }

    const _USERINFO: () = test_userinfo(b"04-2al:x$!~", true);
    const _USERINFO_PERCENT: () = test_userinfo(b"abc%40%19%30%59", true);
    const _USERINFO_EMPTY: () = test_userinfo(b"", false);
    const _USERINFO_RESERVED: () = test_userinfo(b"abc@", false);
    const _USERINFO_NON_ASCII: () = test_userinfo(b"abc\x19", false);

    #[allow(dead_code)]
    const fn test_authority(s: &[u8], expected: Result<usize, AuthorityError>) {
        match (validate_authority_bytes(s), expected) {
            (Ok(x), Ok(y)) if x == y => (),
            (Err(x), Err(y)) if x as u8 == y as u8 => (),
            _ => panic!("authority test failed"),
        }
    }

    // Pass tests
    const _AUTHORITY_FULL: () = test_authority(b"u:p@abc:5?some", Ok(9));
    const _AUTHORITY_NO_PORT: () = test_authority(b"u@abc", Ok(5));
    const _AUTHORITY_PCT_ENCODED: () = test_authority(b"abc.%19", Ok(7));
    const _AUTHORITY_IPV6: () = test_authority(b"[::ffff:1.2.3.4]", Ok(16));
    const _AUTHORITY_IPV6_FULL: () = test_authority(b"user:pass@[::1]:1234", Ok(20));
    // Fail tests
    const _AUTHORITY_EMPTY: () = test_authority(b"/", Err(AuthorityError::InvalidHostname));
    const _AUTHORITY_TWO_PORTS: () =
        test_authority(b"u@ab:c:", Err(AuthorityError::InvalidHostname));
    const _AUTHORITY_TWO_USERINFO: () =
        test_authority(b"u1@u2@abc", Err(AuthorityError::InvalidAtUsage));
    const _AUTHORITY_BAD_BRACKETS1: () =
        test_authority(b"[abc", Err(AuthorityError::InvalidBracketUsage));
    const _AUTHORITY_BAD_BRACKETS2: () =
        test_authority(b"[::1]]", Err(AuthorityError::InvalidBracketUsage));
    const _AUTHORITY_BAD_PERCENT: () =
        test_authority(b"abc%2", Err(AuthorityError::InvalidPercent));
    const _AUTHORITY_BAD_IPV6: () = test_authority(b"[1:2]", Err(AuthorityError::InvalidIpv6));
    const _AUTHORITY_BAD_ZONEID: () =
        test_authority(b"[::%25wl$]", Err(AuthorityError::InvalidZoneId));
    const _AUTHORITY_BAD_PORT: () = test_authority(b"abc:10c", Err(AuthorityError::InvalidPort));

    #[test]
    fn parse_empty_string_is_error() {
        let err = Authority::parse_non_empty(b"").unwrap_err();
        assert_eq!(err.0, ErrorKind::Empty);
    }

    #[test]
    fn equal_to_self_of_same_authority() {
        let authority1: Authority = "example.com".parse().unwrap();
        let authority2: Authority = "EXAMPLE.COM".parse().unwrap();
        assert_eq!(authority1, authority2);
        assert_eq!(authority2, authority1);
    }

    #[test]
    fn not_equal_to_self_of_different_authority() {
        let authority1: Authority = "example.com".parse().unwrap();
        let authority2: Authority = "test.com".parse().unwrap();
        assert_ne!(authority1, authority2);
        assert_ne!(authority2, authority1);
    }

    #[test]
    fn equates_with_a_str() {
        let authority: Authority = "example.com".parse().unwrap();
        assert_eq!(&authority, "EXAMPLE.com");
        assert_eq!("EXAMPLE.com", &authority);
        assert_eq!(authority, "EXAMPLE.com");
        assert_eq!("EXAMPLE.com", authority);
    }

    #[test]
    fn from_static_equates_with_a_str() {
        let authority = Authority::from_static("example.com");
        assert_eq!(authority, "example.com");
    }

    #[test]
    fn not_equal_with_a_str_of_a_different_authority() {
        let authority: Authority = "example.com".parse().unwrap();
        assert_ne!(&authority, "test.com");
        assert_ne!("test.com", &authority);
        assert_ne!(authority, "test.com");
        assert_ne!("test.com", authority);
    }

    #[test]
    fn equates_with_a_string() {
        let authority: Authority = "example.com".parse().unwrap();
        assert_eq!(authority, "EXAMPLE.com".to_string());
        assert_eq!("EXAMPLE.com".to_string(), authority);
    }

    #[test]
    fn equates_with_a_string_of_a_different_authority() {
        let authority: Authority = "example.com".parse().unwrap();
        assert_ne!(authority, "test.com".to_string());
        assert_ne!("test.com".to_string(), authority);
    }

    #[test]
    fn compares_to_self() {
        let authority1: Authority = "abc.com".parse().unwrap();
        let authority2: Authority = "def.com".parse().unwrap();
        assert!(authority1 < authority2);
        assert!(authority2 > authority1);
    }

    #[test]
    fn compares_with_a_str() {
        let authority: Authority = "def.com".parse().unwrap();
        // with ref
        assert!(&authority < "ghi.com");
        assert!("ghi.com" > &authority);
        assert!(&authority > "abc.com");
        assert!("abc.com" < &authority);

        // no ref
        assert!(authority < "ghi.com");
        assert!("ghi.com" > authority);
        assert!(authority > "abc.com");
        assert!("abc.com" < authority);
    }

    #[test]
    fn compares_with_a_string() {
        let authority: Authority = "def.com".parse().unwrap();
        assert!(authority < "ghi.com".to_string());
        assert!("ghi.com".to_string() > authority);
        assert!(authority > "abc.com".to_string());
        assert!("abc.com".to_string() < authority);
    }

    #[test]
    fn allows_percent_in_userinfo() {
        let authority_str = "a%2f:b%2f@example.com";
        let authority: Authority = authority_str.parse().unwrap();
        assert_eq!(authority, authority_str);
    }

    #[test]
    fn allows_ok_percent_in_hostname() {
        let authority_str = "example%2f.com";
        let authority: Authority = authority_str.parse().unwrap();
        assert_eq!(authority, authority_str);

        let authority_str = "a%2f:b%2f@example%2f.com";
        let authority: Authority = authority_str.parse().unwrap();
        assert_eq!(authority, authority_str);
    }

    #[test]
    fn allows_percent_in_ipv6_address() {
        let authority_str = "[fe80::1:2:3:4%25eth0]";
        let result: Authority = authority_str.parse().unwrap();
        assert_eq!(result, authority_str);
    }

    #[test]
    fn rejects_redundant_percent_in_hostname() {
        let err = Authority::parse_non_empty(b"example%46com").unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidAuthority);
    }

    #[test]
    fn rejects_bad_percent_in_hostname() {
        let err = Authority::parse_non_empty(b"example%4zcom").unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidAuthority);
    }

    #[test]
    fn reject_obviously_invalid_ipv6_address() {
        let err = Authority::parse_non_empty(b"[0:1:2:3:4:5:6:7:8:9:10:11:12:13:14]").unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidAuthority);
    }

    #[test]
    fn rejects_invalid_utf8() {
        let err = Authority::try_from([0xc0u8].as_ref()).unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidUriChar);

        let err = Authority::from_shared(Bytes::from_static([0xc0u8].as_ref())).unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidUriChar);
    }

    #[test]
    fn rejects_invalid_use_of_brackets() {
        let err = Authority::parse_non_empty(b"[]@[").unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidAuthority);

        // reject tie-fighter
        let err = Authority::parse_non_empty(b"]o[").unwrap_err();
        assert_eq!(err.0, ErrorKind::InvalidAuthority);
    }
}
