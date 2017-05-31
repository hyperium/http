//! HTTP status codes

use std::fmt;
use std::error::Error;
use std::str::FromStr;

/// An HTTP status code (`status-code` in RFC 7230 et al.).
///
/// This type contains constructor functions for  all common status codes.
/// It allows status codes in the range [0, 65535], as any
/// `u16` integer may be used as a status code for XHR requests. It is
/// recommended to only use values between [100, 599], since only these are
/// defined as valid status codes with a status class by HTTP.
///
/// IANA maintain the [Hypertext Transfer Protocol (HTTP) Status Code
/// Registry](http://www.iana.org/assignments/http-status-codes/http-status-codes.xhtml) which is
/// the source for this enum (with one exception, 418 I'm a teapot, which is
/// inexplicably not in the register).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatusCode(u16);

/// A possible error value when converting a `StatusCode` from a `u16`.
///
/// This type is returned from `StatusCode::from_u16` when the supplied input is
/// less than 100 or greater than 599.
#[derive(Debug)]
pub struct FromU16Error {
    _priv: (),
}

/// A possible error value when converting a `StatusCode` from a `&str`.
///
/// This type is returned from `StatusCode::from_str` when the supplied input is
/// not a valid number, less than 100 or greater than 599.
#[derive(Debug)]
pub struct FromStrError {
    _priv: (),
}

impl StatusCode {
    /// Converts a u16 to a status code.
    ///
    /// The function validates the correctness of the supplied u16. It must be
    /// greater or equal to 100 but less than 600.
    pub fn from_u16(src: u16) -> Result<StatusCode, FromU16Error> {
        if src < 100 || src >= 600 {
            return Err(FromU16Error::new());
        }

        Ok(StatusCode(src))
    }

    /// Converts a &[u8] to a status code
    pub fn from_slice(src: &[u8]) -> Result<StatusCode, FromStrError> {
        if src.len() != 3 {
            return Err(FromStrError::new());
        }

        let a = src[0].wrapping_sub(b'0') as u16;
        let b = src[1].wrapping_sub(b'0') as u16;
        let c = src[1].wrapping_sub(b'0') as u16;

        if a > 9 || b > 9 || c > 9 {
            return Err(FromStrError::new());
        }

        let status = (a * 100) + (b * 10) + c;
        Ok(StatusCode(status))
    }

    /// Check if class is Informational.
    pub fn is_informational(&self) -> bool {
        200 > self.0 && self.0 >= 100
    }

    /// Check if class is Success.
    pub fn is_success(&self) -> bool {
        300 > self.0 && self.0 >= 200
    }

    /// Check if class is Redirection.
    pub fn is_redirection(&self) -> bool {
        400 > self.0 && self.0 >= 300
    }

    /// Check if class is ClientError.
    pub fn is_client_error(&self) -> bool {
        500 > self.0 && self.0 >= 400
    }

    /// Check if class is ServerError.
    pub fn is_server_error(&self) -> bool {
        600 > self.0 && self.0 >= 500
    }
}

/// Formats the status code, *including* the canonical reason.
///
/// ```rust
/// # use http::status;
/// assert_eq!(format!("{}", status::OK), "200 OK");
/// ```
impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", u16::from(*self),
               self.canonical_reason().unwrap_or("<unknown status code>"))
    }
}

impl Default for StatusCode {
    fn default() -> StatusCode {
        OK
    }
}

impl From<StatusCode> for u16 {
    fn from(status: StatusCode) -> u16 {
        status.0
    }
}

impl FromStr for StatusCode {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<StatusCode, FromStrError> {
        let code = try!(s.parse().map_err(|_| FromStrError::new()));
        StatusCode::from_u16(code)
            .map_err(|_| FromStrError::new())
    }
}

impl FromU16Error {
    fn new() -> FromU16Error {
        FromU16Error {
            _priv: (),
        }
    }
}

impl FromStrError {
    fn new() -> FromStrError {
        FromStrError {
            _priv: (),
        }
    }
}

macro_rules! status_codes {
    (
        $(
            $(#[$docs:meta])*
            ($num:expr, $konst:ident, $phrase:expr);
        )+
    ) => {
        $(
            $(#[$docs])*
            pub const $konst: StatusCode = StatusCode($num);
        )+

        impl StatusCode {
            /// Get the standardised `reason-phrase` for this status code.
            ///
            /// This is mostly here for servers writing responses, but could potentially have application
            /// at other times.
            ///
            /// The reason phrase is defined as being exclusively for human readers. You should avoid
            /// deriving any meaning from it at all costs.
            ///
            /// Bear in mind also that in HTTP/2.0 the reason phrase is abolished from transmission, and so
            /// this canonical reason phrase really is the only reason phrase you’ll find.
            pub fn canonical_reason(&self) -> Option<&'static str> {
                match self.0 {
                    $(
                    $num => Some($phrase),
                    )+
                    _ => None
                }
            }
        }
    }
}

status_codes! {
    /// 100 Continue
    /// [[RFC7231, Section 6.2.1](https://tools.ietf.org/html/rfc7231#section-6.2.1)]
    (100, CONTINUE, "Continue");
    /// 101 Switching Protocols
    /// [[RFC7231, Section 6.2.2](https://tools.ietf.org/html/rfc7231#section-6.2.2)]
    (101, SWITCHING_PROTOCOLS, "Switching Protocols");
    /// 102 Processing
    /// [[RFC2518](https://tools.ietf.org/html/rfc2518)]
    (102, PROCESSING, "Processing");

    /// 200 OK
    /// [[RFC7231, Section 6.3.1](https://tools.ietf.org/html/rfc7231#section-6.3.1)]
    (200, OK, "OK");
    /// 201 Created
    /// [[RFC7231, Section 6.3.2](https://tools.ietf.org/html/rfc7231#section-6.3.2)]
    (201, CREATED, "Created");
    /// 202 Accepted
    /// [[RFC7231, Section 6.3.3](https://tools.ietf.org/html/rfc7231#section-6.3.3)]
    (202, ACCEPTED, "Accepted");
    /// 203 Non-Authoritative Information
    /// [[RFC7231, Section 6.3.4](https://tools.ietf.org/html/rfc7231#section-6.3.4)]
    (203, NON_AUTHORITATIVE_INFORMATION, "Non Authoritative Information");
    /// 204 No Content
    /// [[RFC7231, Section 6.3.5](https://tools.ietf.org/html/rfc7231#section-6.3.5)]
    (204, NO_CONTENT, "No Content");
    /// 205 Reset Content
    /// [[RFC7231, Section 6.3.6](https://tools.ietf.org/html/rfc7231#section-6.3.6)]
    (205, RESET_CONTENT, "Reset Content");
    /// 206 Partial Content
    /// [[RFC7233, Section 4.1](https://tools.ietf.org/html/rfc7233#section-4.1)]
    (206, PARTIAL_CONTENT, "Partial Content");
    /// 207 Multi-Status
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    (207, MULTI_STATUS, "Multi-Status");
    /// 208 Already Reported
    /// [[RFC5842](https://tools.ietf.org/html/rfc5842)]
    (208, ALREADY_REPORTED, "Already Reported");

    /// 226 IM Used
    /// [[RFC3229](https://tools.ietf.org/html/rfc3229)]
    (226, IM_USED, "IM Used");

    /// 300 Multiple Choices
    /// [[RFC7231, Section 6.4.1](https://tools.ietf.org/html/rfc7231#section-6.4.1)]
    (300, MULTIPLE_CHOICES, "Multiple Choices");
    /// 301 Moved Permanently
    /// [[RFC7231, Section 6.4.2](https://tools.ietf.org/html/rfc7231#section-6.4.2)]
    (301, MOVED_PERMANENTLY, "Moved Permanently");
    /// 302 Found
    /// [[RFC7231, Section 6.4.3](https://tools.ietf.org/html/rfc7231#section-6.4.3)]
    (302, FOUND, "Found");
    /// 303 See Other
    /// [[RFC7231, Section 6.4.4](https://tools.ietf.org/html/rfc7231#section-6.4.4)]
    (303, SEE_OTHER, "See Other");
    /// 304 Not Modified
    /// [[RFC7232, Section 4.1](https://tools.ietf.org/html/rfc7232#section-4.1)]
    (304, NOT_MODIFIED, "Not Modified");
    /// 305 Use Proxy
    /// [[RFC7231, Section 6.4.5](https://tools.ietf.org/html/rfc7231#section-6.4.5)]
    (305, USE_PROXY, "Use Proxy");
    /// 307 Temporary Redirect
    /// [[RFC7231, Section 6.4.7](https://tools.ietf.org/html/rfc7231#section-6.4.7)]
    (307, TEMPORARY_REDIRECT, "Temporary Redirect");
    /// 308 Permanent Redirect
    /// [[RFC7238](https://tools.ietf.org/html/rfc7238)]
    (308, PERMANENT_REDIRECT, "Permanent Redirect");

    /// 400 Bad Request
    /// [[RFC7231, Section 6.5.1](https://tools.ietf.org/html/rfc7231#section-6.5.1)]
    (400, BAD_REQUEST, "Bad Request");
    /// 401 Unauthorized
    /// [[RFC7235, Section 3.1](https://tools.ietf.org/html/rfc7235#section-3.1)]
    (401, UNAUTHORIZED, "Unauthorized");
    /// 402 Payment Required
    /// [[RFC7231, Section 6.5.2](https://tools.ietf.org/html/rfc7231#section-6.5.2)]
    (402, PAYMENT_REQUIRED, "Payment Required");
    /// 403 Forbidden
    /// [[RFC7231, Section 6.5.3](https://tools.ietf.org/html/rfc7231#section-6.5.3)]
    (403, FORBIDDEN, "Forbidden");
    /// 404 Not Found
    /// [[RFC7231, Section 6.5.4](https://tools.ietf.org/html/rfc7231#section-6.5.4)]
    (404, NOT_FOUND, "Not Found");
    /// 405 Method Not Allowed
    /// [[RFC7231, Section 6.5.5](https://tools.ietf.org/html/rfc7231#section-6.5.5)]
    (405, METHOD_NOT_ALLOWED, "Method Not Allowed");
    /// 406 Not Acceptable
    /// [[RFC7231, Section 6.5.6](https://tools.ietf.org/html/rfc7231#section-6.5.6)]
    (406, NOT_ACCEPTABLE, "Not Acceptable");
    /// 407 Proxy Authentication Required
    /// [[RFC7235, Section 3.2](https://tools.ietf.org/html/rfc7235#section-3.2)]
    (407, PROXY_AUTHENTICATION_REQUIRED, "Proxy Authentication Required");
    /// 408 Request Timeout
    /// [[RFC7231, Section 6.5.7](https://tools.ietf.org/html/rfc7231#section-6.5.7)]
    (408, REQUEST_TIMEOUT, "Request Timeout");
    /// 409 Conflict
    /// [[RFC7231, Section 6.5.8](https://tools.ietf.org/html/rfc7231#section-6.5.8)]
    (409, CONFLICT, "Conflict");
    /// 410 Gone
    /// [[RFC7231, Section 6.5.9](https://tools.ietf.org/html/rfc7231#section-6.5.9)]
    (410, GONE, "Gone");
    /// 411 Length Required
    /// [[RFC7231, Section 6.5.10](https://tools.ietf.org/html/rfc7231#section-6.5.10)]
    (411, LENGTH_REQUIRED, "Length Required");
    /// 412 Precondition Failed
    /// [[RFC7232, Section 4.2](https://tools.ietf.org/html/rfc7232#section-4.2)]
    (412, PRECONDITION_FAILED, "Precondition Failed");
    /// 413 Payload Too Large
    /// [[RFC7231, Section 6.5.11](https://tools.ietf.org/html/rfc7231#section-6.5.11)]
    (413, PAYLOAD_TOO_LARGE, "Payload Too Large");
    /// 414 URI Too Long
    /// [[RFC7231, Section 6.5.12](https://tools.ietf.org/html/rfc7231#section-6.5.12)]
    (414, URI_TOO_LONG, "URI Too Long");
    /// 415 Unsupported Media Type
    /// [[RFC7231, Section 6.5.13](https://tools.ietf.org/html/rfc7231#section-6.5.13)]
    (415, UNSUPPORTED_MEDIA_TYPE, "Unsupported Media Type");
    /// 416 Range Not Satisfiable
    /// [[RFC7233, Section 4.4](https://tools.ietf.org/html/rfc7233#section-4.4)]
    (416, RANGE_NOT_SATISFIABLE, "Range Not Satisfiable");
    /// 417 Expectation Failed
    /// [[RFC7231, Section 6.5.14](https://tools.ietf.org/html/rfc7231#section-6.5.14)]
    (417, EXPECTATION_FAILED, "Expectation Failed");
    /// 418 I'm a teapot
    /// [curiously not registered by IANA but [RF C2324](https://tools.ietf.org/html/rfc2324)]
    (418, IM_A_TEAPOT, "I'm a teapot");

    /// 421 Misdirected Request
    /// [RFC7540, Section 9.1.2](http://tools.ietf.org/html/rfc7540#section-9.1.2)
    (421, MISDIRECTED_REQUEST, "Misdirected Request");
    /// 422 Unprocessable Entity
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    (422, UNPROCESSABLE_ENTITY, "Unprocessable Entity");
    /// 423 Locked
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    (423, LOCKED, "Locked");
    /// 424 Failed Dependency
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    (424, FAILED_DEPENDENCY, "Failed Dependency");

    /// 426 Upgrade Required
    /// [[RFC7231, Section 6.5.15](https://tools.ietf.org/html/rfc7231#section-6.5.15)]
    (426, UPGRADE_REQUIRED, "Upgrade Required");

    /// 428 Precondition Required
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    (428, PRECONDITION_REQUIRED, "Precondition Required");
    /// 429 Too Many Requests
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    (429, TOO_MANY_REQUESTS, "Too Many Requests");

    /// 431 Request Header Fields Too Large
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    (431, REQUEST_HEADER_FIELDS_TOO_LARGE, "Request Header Fields Too Large");

    /// 451 Unavailable For Legal Reasons
    /// [[RFC7725](http://tools.ietf.org/html/rfc7725)]
    (451, UNAVAILABLE_FOR_LEGAL_REASONS, "Unavailable For Legal Reasons");

    /// 500 Internal Server Error
    /// [[RFC7231, Section 6.6.1](https://tools.ietf.org/html/rfc7231#section-6.6.1)]
    (500, INTERNAL_SERVER_ERROR, "Internal Server Error");
    /// 501 Not Implemented
    /// [[RFC7231, Section 6.6.2](https://tools.ietf.org/html/rfc7231#section-6.6.2)]
    (501, NOT_IMPLEMENTED, "Not Implemented");
    /// 502 Bad Gateway
    /// [[RFC7231, Section 6.6.3](https://tools.ietf.org/html/rfc7231#section-6.6.3)]
    (502, BAD_GATEWAY, "Bad Gateway");
    /// 503 Service Unavailable
    /// [[RFC7231, Section 6.6.4](https://tools.ietf.org/html/rfc7231#section-6.6.4)]
    (503, SERVICE_UNAVAILABLE, "Service Unavailable");
    /// 504 Gateway Timeout
    /// [[RFC7231, Section 6.6.5](https://tools.ietf.org/html/rfc7231#section-6.6.5)]
    (504, GATEWAY_TIMEOUT, "Gateway Timeout");
    /// 505 HTTP Version Not Supported
    /// [[RFC7231, Section 6.6.6](https://tools.ietf.org/html/rfc7231#section-6.6.6)]
    (505, HTTP_VERSION_NOT_SUPPORTED, "HTTP Version Not Supported");
    /// 506 Variant Also Negotiates
    /// [[RFC2295](https://tools.ietf.org/html/rfc2295)]
    (506, VARIANT_ALSO_NEGOTIATES, "Variant Also Negotiates");
    /// 507 Insufficient Storage
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    (507, INSUFFICIENT_STORAGE, "Insufficient Storage");
    /// 508 Loop Detected
    /// [[RFC5842](https://tools.ietf.org/html/rfc5842)]
    (508, LOOP_DETECTED, "Loop Detected");

    /// 510 Not Extended
    /// [[RFC2774](https://tools.ietf.org/html/rfc2774)]
    (510, NOT_EXTENDED, "Not Extended");
    /// 511 Network Authentication Required
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    (511, NETWORK_AUTHENTICATION_REQUIRED, "Network Authentication Required");
}

impl fmt::Display for FromU16Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for FromU16Error {
    fn description(&self) -> &str {
        "invalid status code"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for FromStrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for FromStrError {
    fn description(&self) -> &str {
        "invalid status code"
    }
}
