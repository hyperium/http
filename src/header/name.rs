use byte_str::ByteStr;

use bytes::{Bytes, BytesMut};

use std::mem;

#[derive(Clone, Eq, PartialEq)]
pub struct HeaderName {
    inner: Repr,
}

#[derive(Clone, Eq, PartialEq)]
enum Repr {
    Standard(StandardHeader),
    Custom(ByteStr),
}

#[derive(Debug)]
pub struct FromBytesError {
    _priv: (),
}

macro_rules! standard_headers {
    (
        $(
            $(#[$docs:meta])*
            ($konst:ident, $upcase:ident, $name:expr);
        )+
    ) => {
        #[derive(Debug, Clone, Copy, Eq, PartialEq)]
        enum StandardHeader {
            $(
                $konst,
            )+
        }

        $(
            $(#[$docs])*
            pub const $upcase: HeaderName = HeaderName {
                inner: Repr::Standard(StandardHeader::$konst),
            };
        )+

        impl StandardHeader {
            fn as_str(&self) -> &'static str {
                match *self {
                    $(
                    StandardHeader::$konst => $name,
                    )+
                }
            }
        }
    }
}

standard_headers! {
    /// Advertises which content types the client is able to understand.
    ///
    /// The Accept request HTTP header advertises which content types, expressed
    /// as MIME types, the client is able to understand. Using content
    /// negotiation, the server then selects one of the proposals, uses it and
    /// informs the client of its choice with the Content-Type response header.
    /// Browsers set adequate values for this header depending of the context
    /// where the request is done: when fetching a CSS stylesheet a different
    /// value is set for the request than when fetching an image, video or a
    /// script.
    (Accept, ACCEPT, "accept");

    /// Advertises which character set the client is able to understand.
    ///
    /// The Accept-Charset request HTTP header advertises which character set
    /// the client is able to understand. Using content negotiation, the server
    /// then selects one of the proposals, uses it and informs the client of its
    /// choice within the Content-Type response header. Browsers usually don't
    /// set this header as the default value for each content type is usually
    /// correct and transmitting it would allow easier fingerprinting.
    ///
    /// If the server cannot serve any matching character set, it can
    /// theoretically send back a 406 (Not Acceptable) error code. But, for a
    /// better user experience, this is rarely done and the more common way is
    /// to ignore the Accept-Charset header in this case.
    (AcceptCharset, ACCEPT_CHARSET, "accept-charset");

    /// The Accept-Encoding request HTTP header advertises which content
    /// encoding, usually a compression algorithm, the client is able to
    /// understand. Using content negotiation, the server selects one of the
    /// proposals, uses it and informs the client of its choice with the
    /// Content-Encoding response header.
    ///
    /// Even if both the client and the server supports the same compression
    /// algorithms, the server may choose not to compress the body of a
    /// response, if the identity value is also acceptable. Two common cases
    /// lead to this:
    ///
    /// * The data to be sent is already compressed and a second compression
    /// won't lead to smaller data to be transmitted. This may the case with
    /// some image formats;
    ///
    /// * The server is overloaded and cannot afford the computational overhead
    /// induced by the compression requirement. Typically, Microsoft recommends
    /// not to compress if a server use more than 80 % of its computational
    /// power.
    ///
    /// As long as the identity value, meaning no encryption, is not explicitly
    /// forbidden, by an identity;q=0 or a *;q=0 without another explicitly set
    /// value for identity, the server must never send back a 406 Not Acceptable
    /// error.
    (AcceptEncoding, ACCEPT_ENCODING, "accept-encoding");

    /// Content-Types that are acceptable for the response.
    (AcceptLanguage, ACCEPT_LANGUAGE, "accept-language");

    /// Content-Types that are acceptable for the response.
    (AcceptDatetime, ACCEPT_DATETIME, "accept-datetime");

    /// Content-Types that are acceptable for the response.
    (AcceptPatch, ACCEPT_PATCH, "accept-patch");

    /// Content-Types that are acceptable for the response.
    (AcceptRanges, ACCEPT_RANGES, "accept-ranges");

    /// Content-Types that are acceptable for the response.
    (AccessControlAllowOrigin, ACCESS_CONTROL_ALLOW_ORIGIN, "access-control-allow-origin");

    /// Content-Types that are acceptable for the response.
    (Age, AGE, "age");

    /// Content-Types that are acceptable for the response.
    (Allow, ALLOW, "allow");

    /// Content-Types that are acceptable for the response.
    (AltSvc, ALT_SVC, "alt-svc");

    /// Content-Types that are acceptable for the response.
    (Authorization, AUTHORIZATION, "authorization");

    /// Content-Types that are acceptable for the response.
    (CacheControl, CACHE_CONTROL, "cache-control");

    /// Content-Types that are acceptable for the response.
    (Cookie, COOKIE, "cookie");

    /// Content-Types that are acceptable for the response.
    (Connection, CONNECTION, "connection");

    /// Content-Types that are acceptable for the response.
    (ContentDisposition, CONTENT_DISPOSITION, "content-disposition");

    /// Content-Types that are acceptable for the response.
    (ContentEncoding, CONTENT_ENCODING, "content-encoding");

    /// Content-Types that are acceptable for the response.
    (ContentLanguage, CONTENT_LANGUAGE, "content-language");

    /// Content-Types that are acceptable for the response.
    (ContentLength, CONTENT_LENGTH, "content-length");

    /// Content-Types that are acceptable for the response.
    (ContentLocation, CONTENT_LOCATION, "content-location");

    /// Content-Types that are acceptable for the response.
    (ContentMd5, CONTENT_MD5, "content-md5");

    /// Content-Types that are acceptable for the response.
    (ContentRange, CONTENT_RANGE, "content-range");

    /// Content-Types that are acceptable for the response.
    (ContentType, CONTENT_TYPE, "content-type");

    /// Content-Types that are acceptable for the response.
    (Date, DATE, "date");

    /// Content-Types that are acceptable for the response.
    (Etag, ETAG, "etag");

    /// Content-Types that are acceptable for the response.
    (Expect, EXPECT, "expect");

    /// Content-Types that are acceptable for the response.
    (Expires, EXPIRES, "expires");

    /// Content-Types that are acceptable for the response.
    (Forwarded, FORWARDED, "forwarded");

    /// Content-Types that are acceptable for the response.
    (From, FROM, "from");

    /// Content-Types that are acceptable for the response.
    (Host, HOST, "host");

    /// Content-Types that are acceptable for the response.
    (IfMatch, IF_MATCH, "if-match");

    /// Content-Types that are acceptable for the response.
    (IfModifiedSince, IF_MODIFIED_SINCE, "if-modified-since");

    /// Content-Types that are acceptable for the response.
    (IfNoneMatch, IF_NONE_MATCH, "if-none-match");

    /// Content-Types that are acceptable for the response.
    (IfRange, IF_RANGE, "if-range");

    /// Content-Types that are acceptable for the response.
    (IfUnmodifiedSince, IF_UNMODIFIED_SINCE, "if-unmodified-since");

    /// Content-Types that are acceptable for the response.
    (LastModified, LAST_MODIFIED, "last-modified");

    /// Content-Types that are acceptable for the response.
    (Link, LINK, "link");

    /// Content-Types that are acceptable for the response.
    (Location, LOCATION, "location");

    /// Content-Types that are acceptable for the response.
    (MaxForwards, MAX_FORWARDS, "max-forwards");

    /// Content-Types that are acceptable for the response.
    (Origin, ORIGIN, "origin");

    /// Content-Types that are acceptable for the response.
    (P3p, P3P, "p3p");

    /// Content-Types that are acceptable for the response.
    (Pragma, PRAGMA, "pragma");

    /// Content-Types that are acceptable for the response.
    (ProxyAuthenticate, PROXY_AUTHENTICATE, "proxy-authenticate");

    /// Content-Types that are acceptable for the response.
    (ProxyAuthorization, PROXY_AUTHORIZATION, "proxy-authorization");

    /// Content-Types that are acceptable for the response.
    (PublicKeyPins, PUBLIC_KEY_PINS, "public-key-pins");

    /// Content-Types that are acceptable for the response.
    (Range, RANGE, "range");

    /// Content-Types that are acceptable for the response.
    (Referer, REFERER, "referer");

    /// Content-Types that are acceptable for the response.
    (Refresh, REFRESH, "refresh");

    /// Content-Types that are acceptable for the response.
    (RetryAfter, RETRY_AFTER, "retry-after");

    /// Content-Types that are acceptable for the response.
    (Server, SERVER, "server");

    /// Content-Types that are acceptable for the response.
    (SetCookie, SET_COOKIE, "set-cookie");

    /// Content-Types that are acceptable for the response.
    (Status, STATUS, "status");

    /// Content-Types that are acceptable for the response.
    (StrictTransportSecurity, STRICT_TRANSPORT_SECURITY, "strict-transport-security");

    /// Content-Types that are acceptable for the response.
    (Te, TE, "te");

    /// Content-Types that are acceptable for the response.
    (Trailer, TRAILER, "trailer");

    /// Content-Types that are acceptable for the response.
    (TransferEncoding, TRANSFER_ENCODING, "transfer-encoding");

    /// Content-Types that are acceptable for the response.
    (Tsv, TSV, "tsv");

    /// Content-Types that are acceptable for the response.
    (UserAgent, USER_AGENT, "user-agent");

    /// Content-Types that are acceptable for the response.
    (Upgrade, UPGRADE, "upgrade");

    /// Content-Types that are acceptable for the response.
    (Vary, VARY, "vary");

    /// Content-Types that are acceptable for the response.
    (Via, VIA, "via");

    /// Content-Types that are acceptable for the response.
    (Warning, WARNING, "warning");

    /// Content-Types that are acceptable for the response.
    (Warnings, WARNINGS, "warnings");

    /// Content-Types that are acceptable for the response.
    (WwwAuthenticate, WWW_AUTHENTICATE, "www-authenticate");

    /// Content-Types that are acceptable for the response.
    (XFrameOptions, X_FRAME_OPTIONS, "x-frame-options");
}

/*
#[derive(Clone, Copy, Eq, PartialEq)]
enum StandardHeader {
    Accept,
    AcceptCharset,
    AcceptEncoding,
    AcceptLanguage,
    AcceptDatetime,
    AcceptPatch,
    AcceptRanges,
    AccessControlAllowOrigin,
    Age,
    Allow,
    AltSvc,
    Authorization,
    CacheControl,
    Cookie,
    Connection,
    ContentDisposition,
    ContentEncoding,
    ContentLanguage,
    ContentLength,
    ContentLocation,
    ContentMd5,
    ContentRange,
    ContentType,
    Date,
    Etag,
    Expect,
    Expires,
    Forwarded,
    From,
    Host,
    IfMatch,
    IfModifiedSince,
    IfNoneMatch,
    IfRange,
    IfUnmodifiedSince,
    LastModified,
    Link,
    Location,
    MaxForwards,
    Origin,
    P3p,
    Pragma,
    ProxyAuthenticate,
    ProxyAuthorization,
    PublicKeyPins,
    Range,
    Referer,
    Refresh,
    RetryAfter,
    Server,
    SetCookie,
    Status,
    StrictTransportSecurity,
    Te,
    Trailer,
    TransferEncoding,
    Tsv,
    UserAgent,
    Upgrade,
    Vary,
    Via,
    Warning,
    Warnings,
    WwwAuthenticate,
    XFrameOptions,
}
*/

// pub const ACCEPT: HeaderName = HeaderName { inner: StandardHeader::Accept };

/// Valid header name characters
///
///       field-name     = token
///       token          = 1*<any CHAR except CTLs or separators>
///       separators     = "(" | ")" | "<" | ">" | "@"
///                      | "," | ";" | ":" | "\" | <">
///                      | "/" | "[" | "]" | "?" | "="
///                      | "{" | "}" | SP | HT
const HEADER_CHARS: [u8; 256] = [
    //  0      1      2      3      4      5      6      7      8      9
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //   x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  1x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  2x
        0,     0,     0,  b'!',  b'"',  b'#',  b'$',  b'%',  b'&', b'\'', //  3x
        0,     0,  b'*',  b'+',     0,  b'-',  b'.',     0,  b'0',  b'1', //  4x
     b'2',  b'3',  b'4',  b'5',  b'6',  b'7',  b'8',  b'9',     0,     0, //  5x
        0,     0,     0,     0,     0,  b'a',  b'b',  b'c',  b'd',  b'e', //  6x
     b'f',  b'g',  b'h',  b'i',  b'j',  b'k',  b'l',  b'm',  b'n',  b'o', //  7x
     b'p',  b'q',  b'r',  b's',  b't',  b'u',  b'v',  b'w',  b'x',  b'y', //  8x
     b'z',     0,     0,     0,     0,  b'_',     0,  b'a',  b'b',  b'c', //  9x
     b'd',  b'e',  b'f',  b'g',  b'h',  b'i',  b'j',  b'k',  b'l',  b'm', // 10x
     b'n',  b'o',  b'p',  b'q',  b'r',  b's',  b't',  b'u',  b'v',  b'w', // 11x
     b'x',  b'y',  b'z',     0,  b'|',     0,  b'~',     0,     0,     0, // 12x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 13x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 14x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 15x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 16x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 17x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 18x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 19x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 20x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 21x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 22x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 23x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 24x
        0,     0,     0,     0,     0,     0                              // 25x
];

macro_rules! eq {
    ($v:ident[$n:expr] == $a:tt) => {
        $v[$n] == $a
    };
    ($v:ident[$n:expr] == $a:tt $($rest:tt)+) => {
        $v[$n] == $a && eq!($v[($n+1)] == $($rest)+)
    };
    ($v:ident == $a:tt $($rest:tt)*) => {
        $v[0] == $a && eq!($v[1] == $($rest)*)
    };
}

macro_rules! to_lower {
    ($d:ident, $src:ident, 1) => { $d[0] = HEADER_CHARS[$src[0] as usize]; };
    ($d:ident, $src:ident, 2) => { to_lower!($d, $src, 1); $d[1] = HEADER_CHARS[$src[1] as usize]; };
    ($d:ident, $src:ident, 3) => { to_lower!($d, $src, 2); $d[2] = HEADER_CHARS[$src[2] as usize]; };
    ($d:ident, $src:ident, 4) => { to_lower!($d, $src, 3); $d[3] = HEADER_CHARS[$src[3] as usize]; };
    ($d:ident, $src:ident, 5) => { to_lower!($d, $src, 4); $d[4] = HEADER_CHARS[$src[4] as usize]; };
    ($d:ident, $src:ident, 6) => { to_lower!($d, $src, 5); $d[5] = HEADER_CHARS[$src[5] as usize]; };
    ($d:ident, $src:ident, 7) => { to_lower!($d, $src, 6); $d[6] = HEADER_CHARS[$src[6] as usize]; };
    ($d:ident, $src:ident, 8) => { to_lower!($d, $src, 7); $d[7] = HEADER_CHARS[$src[7] as usize]; };
    ($d:ident, $src:ident, 9) => { to_lower!($d, $src, 8); $d[8] = HEADER_CHARS[$src[8] as usize]; };
    ($d:ident, $src:ident, 10) => { to_lower!($d, $src, 9); $d[9] = HEADER_CHARS[$src[9] as usize]; };
    ($d:ident, $src:ident, 11) => { to_lower!($d, $src, 10); $d[10] = HEADER_CHARS[$src[10] as usize]; };
    ($d:ident, $src:ident, 12) => { to_lower!($d, $src, 11); $d[11] = HEADER_CHARS[$src[11] as usize]; };
    ($d:ident, $src:ident, 13) => { to_lower!($d, $src, 12); $d[12] = HEADER_CHARS[$src[12] as usize]; };
    ($d:ident, $src:ident, 14) => { to_lower!($d, $src, 13); $d[13] = HEADER_CHARS[$src[13] as usize]; };
    ($d:ident, $src:ident, 15) => { to_lower!($d, $src, 14); $d[14] = HEADER_CHARS[$src[14] as usize]; };
    ($d:ident, $src:ident, 16) => { to_lower!($d, $src, 15); $d[15] = HEADER_CHARS[$src[15] as usize]; };
    ($d:ident, $src:ident, 17) => { to_lower!($d, $src, 16); $d[16] = HEADER_CHARS[$src[16] as usize]; };
    ($d:ident, $src:ident, 18) => { to_lower!($d, $src, 17); $d[17] = HEADER_CHARS[$src[17] as usize]; };
    ($d:ident, $src:ident, 19) => { to_lower!($d, $src, 18); $d[18] = HEADER_CHARS[$src[18] as usize]; };
    ($d:ident, $src:ident, 20) => { to_lower!($d, $src, 19); $d[19] = HEADER_CHARS[$src[19] as usize]; };
    ($d:ident, $src:ident, 21) => { to_lower!($d, $src, 20); $d[20] = HEADER_CHARS[$src[20] as usize]; };
    ($d:ident, $src:ident, 22) => { to_lower!($d, $src, 21); $d[21] = HEADER_CHARS[$src[21] as usize]; };
    ($d:ident, $src:ident, 23) => { to_lower!($d, $src, 22); $d[22] = HEADER_CHARS[$src[22] as usize]; };
    ($d:ident, $src:ident, 24) => { to_lower!($d, $src, 23); $d[23] = HEADER_CHARS[$src[23] as usize]; };
    ($d:ident, $src:ident, 25) => { to_lower!($d, $src, 24); $d[24] = HEADER_CHARS[$src[24] as usize]; };
    ($d:ident, $src:ident, 26) => { to_lower!($d, $src, 25); $d[25] = HEADER_CHARS[$src[25] as usize]; };
    ($d:ident, $src:ident, 27) => { to_lower!($d, $src, 26); $d[26] = HEADER_CHARS[$src[26] as usize]; };
}

macro_rules! validate_chars {
    ($buf:ident) => {{
        if $buf.iter().any(|&b| b == 0) {
            return Err(FromBytesError::new());
        }
    }};
}

macro_rules! parse_hdr {
    ($data:ident, $res:ident, $standard:expr, $short:expr, $long: expr) => {{
        use self::StandardHeader::*;

        let len = $data.len();

        match len {
            2 => {
                let mut b: [u8; 2] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 2);

                if eq!(b == b't' b'e') {
                    let $res = Te;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            3 => {
                let mut b: [u8; 3] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 3);

                if eq!(b == b'a' b'g' b'e') {
                    let $res = Age;
                    return $standard;
                } else if eq!(b == b'p' b'3' b'p') {
                    let $res = P3p;
                    return $standard;
                } else if eq!(b == b't' b's' b'v') {
                    let $res = Tsv;
                    return $standard;
                } else if eq!(b == b'v' b'i' b'a') {
                    let $res = Via;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            4 => {
                let mut b: [u8; 4] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 4);

                if eq!(b == b'd' b'a' b't' b'e') {
                    let $res = Date;
                    return $standard;
                } else if eq!(b == b'e' b't' b'a' b'g') {
                    let $res = Etag;
                    return $standard;
                } else if eq!(b == b'f' b'r' b'o' b'm') {
                    let $res = From;
                    return $standard;
                } else if eq!(b == b'h' b'o' b's' b't') {
                    let $res = Host;
                    return $standard;
                } else if eq!(b == b'l' b'i' b'n' b'k') {
                    let $res = Link;
                    return $standard;
                } else if eq!(b == b'v' b'a' b'r' b'y') {
                    let $res = Vary;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            5 => {
                let mut b: [u8; 5] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 5);

                if eq!(b == b'a' b'l' b'l' b'o' b'w') {
                    let $res = Allow;
                    return $standard;
                } else if eq!(b == b'r' b'a' b'n' b'g' b'e') {
                    let $res = Range;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            6 => {
                let mut b: [u8; 6] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 6);

                if eq!(b == b'a' b'c' b'c' b'e' b'p' b't') {
                    let $res = Accept;
                    return $standard;
                } else if eq!(b == b'c' b'o' b'o' b'k' b'i' b'e') {
                    let $res = Cookie;
                    return $standard;
                } else if eq!(b == b'e' b'x' b'p' b'e' b'c' b't') {
                    let $res = Expect;
                    return $standard;
                } else if eq!(b == b'o' b'r' b'i' b'g' b'i' b'n') {
                    let $res = Origin;
                    return $standard;
                } else if eq!(b == b'p' b'r' b'a' b'g' b'm' b'a') {
                    let $res = Pragma;
                    return $standard;
                } if b[0] == b's' {
                    if eq!(b[1] == b'e' b'r' b'v' b'e' b'r') {
                        let $res = Server;
                        return $standard;
                    } else if eq!(b[1] == b't' b'a' b't' b'u' b's') {
                        let $res = Status;
                        return $standard;
                    }
                }

                {
                    let $res = &b[..];
                    return $short;
                }
            }
            7 => {
                let mut b: [u8; 7] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 7);

                if eq!(b == b'a' b'l' b't' b'-' b's' b'v' b'c') {
                    let $res = AltSvc;
                    return $standard;
                } else if eq!(b == b'e' b'x' b'p' b'i' b'r' b'e' b's') {
                    let $res = Expires;
                    return $standard;
                } else if eq!(b == b'r' b'e' b'f' b'e' b'r' b'e' b'r') {
                    let $res = Referer;
                    return $standard;
                } else if eq!(b == b'r' b'e' b'f' b'r' b'e' b's' b'h') {
                    let $res = Refresh;
                    return $standard;
                } else if eq!(b == b't' b'r' b'a' b'i' b'l' b'e' b'r') {
                    let $res = Trailer;
                    return $standard;
                } else if eq!(b == b'u' b'p' b'g' b'r' b'a' b'd' b'e') {
                    let $res = Upgrade;
                    return $standard;
                } else if eq!(b == b'w' b'a' b'r' b'n' b'i' b'n' b'g') {
                    let $res = Warning;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            8 => {
                let mut b: [u8; 8] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 8);

                if eq!(b == b'i' b'f' b'-') {
                    if eq!(b[3] == b'm' b'a' b't' b'c' b'h') {
                        let $res = IfMatch;
                        return $standard;
                    } else if eq!(b[3] == b'r' b'a' b'n' b'g' b'e') {
                        let $res = IfRange;
                        return $standard;
                    }
                } else if eq!(b == b'l' b'o' b'c' b'a' b't' b'i' b'o' b'n') {
                    let $res = Location;
                    return $standard;
                } else if eq!(b == b'w' b'a' b'r' b'n' b'i' b'n' b'g' b's') {
                    let $res = Warnings;
                    return $standard;
                }

                {
                    let $res = &b[..];
                    return $short;
                }
            }
            9 => {
                let mut b: [u8; 9] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 9);

                if eq!(b == b'f' b'o' b'r' b'w' b'a' b'r' b'd' b'e' b'd') {
                    let $res = Forwarded;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            10 => {
                let mut b: [u8; 10] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 10);

                if eq!(b == b'c' b'o' b'n' b'n' b'e' b'c' b't' b'i' b'o' b'n') {
                    let $res = Connection;
                    return $standard;
                } else if eq!(b == b's' b'e' b't' b'-' b'c' b'o' b'o' b'k' b'i' b'e') {
                    let $res = SetCookie;
                    return $standard;
                } else if eq!(b == b'u' b's' b'e' b'r' b'-' b'a' b'g' b'e' b'n' b't') {
                    let $res = UserAgent;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            11 => {
                let mut b: [u8; 11] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 11);

                if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-' b'm' b'd' b'5') {
                    let $res = ContentMd5;
                    return $standard;
                } else if eq!(b == b'r' b'e' b't' b'r' b'y' b'-' b'a' b'f' b't' b'e' b'r') {
                    let $res = RetryAfter;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            12 => {
                let mut b: [u8; 12] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 12);

                if eq!(b == b'a' b'c' b'c' b'e' b'p' b't' b'-' b'p' b'a' b't' b'c' b'h') {
                    let $res = AcceptPatch;
                    return $standard;
                } else if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-' b't' b'y' b'p' b'e') {
                    let $res = ContentType;
                    return $standard;
                } else if eq!(b == b'm' b'a' b'x' b'-' b'f' b'o' b'r' b'w' b'a' b'r' b'd' b's') {
                    let $res = MaxForwards;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            13 => {
                let mut b: [u8; 13] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 13);

                if b[0] == b'a' {
                    if eq!(b[1] == b'c' b'c' b'e' b'p' b't' b'-' b'r' b'a' b'n' b'g' b'e' b's') {
                        let $res = AcceptRanges;
                        return $standard;
                    } else if eq!(b[1] == b'u' b't' b'h' b'o' b'r' b'i' b'z' b'a' b't' b'i' b'o' b'n') {
                        let $res = Authorization;
                        return $standard;
                    }
                } else if b[0] == b'c' {
                    if eq!(b[1] == b'a' b'c' b'h' b'e' b'-' b'c' b'o' b'n' b't' b'r' b'o' b'l') {
                        let $res = CacheControl;
                        return $standard;
                    } else if eq!(b[1] == b'o' b'n' b't' b'e' b'n' b't' b'-' b'r' b'a' b'n' b'g' b'e' ) {
                        let $res = ContentRange;
                        return $standard;
                    }
                } else if eq!(b == b'i' b'f' b'-' b'n' b'o' b'n' b'e' b'-' b'm' b'a' b't' b'c' b'h') {
                    let $res = IfNoneMatch;
                    return $standard;
                } else if eq!(b == b'l' b'a' b's' b't' b'-' b'm' b'o' b'd' b'i' b'f' b'i' b'e' b'd') {
                    let $res = LastModified;
                    return $standard;
                }

                {
                    let $res = &b[..];
                    return $short;
                }
            }
            14 => {
                let mut b: [u8; 14] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 14);

                if eq!(b == b'a' b'c' b'c' b'e' b'p' b't' b'-' b'c' b'h' b'a' b'r' b's' b'e' b't') {
                    let $res = AcceptCharset;
                    return $standard;
                } else if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-' b'l' b'e' b'n' b'g' b't' b'h') {
                    let $res = ContentLength;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            15 => {
                let mut b: [u8; 15] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 15);

                if eq!(b == b'a' b'c' b'c' b'e' b'p' b't' b'-') { // accept-
                    if eq!(b[7] == b'e' b'n' b'c' b'o' b'd' b'i' b'n' b'g') {
                        let $res = AcceptEncoding;
                        return $standard;
                    } else if eq!(b[7] == b'l' b'a' b'n' b'g' b'u' b'a' b'g' b'e') {
                        let $res = AcceptLanguage;
                        return $standard;
                    } else if eq!(b[7] == b'd' b'a' b't' b'e' b't' b'i' b'm' b'e') {
                        let $res = AcceptDatetime;
                        return $standard;
                    }
                } else if eq!(b == b'p' b'u' b'b' b'l' b'i' b'c' b'-' b'k' b'e' b'y' b'-' b'p' b'i' b'n' b's') {
                    let $res = PublicKeyPins;
                    return $standard;
                } else if eq!(b == b'x' b'-' b'f' b'r' b'a' b'm' b'e' b'-' b'o' b'p' b't' b'i' b'o' b'n' b's') {
                    let $res = XFrameOptions;
                    return $standard;
                }

                {
                    let $res = &b[..];
                    return $short;
                }
            }
            16 => {
                let mut b: [u8; 16] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 16);

                if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-') {
                    if eq!(b[8] == b'l' b'a' b'n' b'g' b'u' b'a' b'g' b'e') {
                        let $res = ContentLanguage;
                        return $standard;
                    } else if eq!(b[8] == b'l' b'o' b'c' b'a' b't' b'i' b'o' b'n') {
                        let $res = ContentLocation;
                        return $standard;
                    } else if eq!(b[8] == b'e' b'n' b'c' b'o' b'd' b'i' b'n' b'g') {
                        let $res = ContentEncoding;
                        return $standard;
                    }
                } else if eq!(b == b'w' b'w' b'w' b'-' b'a' b'u' b't' b'h' b'e' b'n' b't' b'i' b'c' b'a' b't' b'e') {
                    let $res = WwwAuthenticate;
                    return $standard;
                }

                {
                    let $res = &b[..];
                    return $short;
                }
            }
            17 => {
                let mut b: [u8; 17] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 17);

                if eq!(b == b't' b'r' b'a' b'n' b's' b'f' b'e' b'r' b'-' b'e' b'n' b'c' b'o' b'd' b'i' b'n' b'g') {
                    let $res = TransferEncoding;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            18 => {
                let mut b: [u8; 18] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 18);

                if eq!(b == b'p' b'r' b'o' b'x' b'y' b'-' b'a' b'u' b't' b'h' b'e' b'n' b't' b'i' b'c' b'a' b't' b'e') {
                    let $res = ProxyAuthenticate;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            19 => {
                let mut b: [u8; 19] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 19);

                if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-' b'd' b'i' b's' b'p' b'o' b's' b'i' b't' b'i' b'o' b'n') {
                    let $res = ContentDisposition;
                    return $standard;
                } else if eq!(b == b'i' b'f' b'-' b'u' b'n' b'm' b'o' b'd' b'i' b'f' b'i' b'e' b'd' b'-' b's' b'i' b'n' b'c' b'e') {
                    let $res = IfUnmodifiedSince;
                    return $standard;
                } else if eq!(b == b'p' b'r' b'o' b'x' b'y' b'-' b'a' b'u' b't' b'h' b'o' b'r' b'i' b'z' b'a' b't' b'i' b'o' b'n') {
                    let $res = ProxyAuthorization;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            25 => {
                let mut b: [u8; 25] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 25);

                if eq!(b == b's' b't' b'r' b'i' b'c' b't' b'-' b't' b'r' b'a' b'n' b's' b'p' b'o' b'r' b't' b'-' b's' b'e' b'c' b'u' b'r' b'i' b't' b'y') {
                    let $res = StrictTransportSecurity;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            27 => {
                let mut b: [u8; 27] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 27);

                if eq!(b == b'a' b'c' b'c' b'e' b's' b's' b'-' b'c' b'o' b'n' b't' b'r' b'o' b'l' b'-' b'a' b'l' b'l' b'o' b'w' b'-' b'o' b'r' b'i' b'g' b'i' b'n') {
                    let $res = AccessControlAllowOrigin;
                    return $standard;
                } else {
                    let $res = &b[..];
                    return $short;
                }
            }
            _ => {
                if 0 == len & !(32-1) {
                    let mut buf: [u8; 31] = unsafe { ::std::mem::uninitialized() };

                    for i in 0..len {
                        buf[i] = HEADER_CHARS[$data[i] as usize];
                    }

                    let $res = &buf[..len];
                    return $short;
                } else {
                    let $res = $data;
                    return $long;
                }
            }
        }
    }};
}

impl HeaderName {
    /// Converts a slice of bytes to an HTTP header name.
    ///
    /// This function normalizes the input.
    pub fn from_bytes(src: &[u8]) -> Result<HeaderName, FromBytesError> {
        parse_hdr!(
            src,
            res,
            Ok(res.into()),
            {
                let buf = Bytes::from(&res[..]);
                let val = unsafe { ByteStr::from_utf8_unchecked(buf) };
                Ok(Repr::Custom(val).into())
            },
            {
                use bytes::{BufMut};
                let mut dst = BytesMut::with_capacity(res.len());

                for b in res.iter() {
                    let b = HEADER_CHARS[*b as usize];

                    if b == 0 {
                        return Err(FromBytesError::new());
                    }

                    dst.put(b);
                }

                let val = unsafe { ByteStr::from_utf8_unchecked(dst.freeze()) };

                Ok(Repr::Custom(val).into())
            })
    }

    /// Returns a `str` representation of the header.
    ///
    /// The returned string will always be lower case.
    pub fn as_str(&self) -> &str {
        match self.inner {
            Repr::Standard(v) => v.as_str(),
            Repr::Custom(ref v) => &**v,
        }
    }
}

impl FromBytesError {
    fn new() -> FromBytesError {
        FromBytesError { _priv: () }
    }
}

impl From<StandardHeader> for HeaderName {
    fn from(src: StandardHeader) -> HeaderName {
        Repr::Standard(src).into()
    }
}

impl From<Repr> for HeaderName {
    fn from(src: Repr) -> HeaderName {
        HeaderName { inner: src }
    }
}
