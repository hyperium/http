//! Constant patterns are part of these types' public API. Equality assertions
//! alone do not catch a representation change that breaks pattern matching.

use http::header::{HeaderName, ACCEPT, CONTENT_TYPE, HOST};
use http::{Method, StatusCode, Version};

#[test]
fn method_constants() {
    for name in &["GET", "POST", "CUSTOM"] {
        let method = Method::from_bytes(name.as_bytes()).unwrap();
        let matched = match method {
            Method::GET => "GET",
            Method::POST => "POST",
            _ => "CUSTOM",
        };
        assert_eq!(matched, *name);
    }

    assert!(matches!(&Method::GET, &Method::GET));
}

#[test]
fn status_code_constants() {
    for code in &[100, 200, 301, 404, 500, 599] {
        let status = StatusCode::from_u16(*code).unwrap();
        let matched = match status {
            StatusCode::CONTINUE => 100,
            StatusCode::OK => 200,
            StatusCode::MOVED_PERMANENTLY => 301,
            StatusCode::NOT_FOUND => 404,
            StatusCode::INTERNAL_SERVER_ERROR => 500,
            _ => 599,
        };
        assert_eq!(matched, *code);
    }

    assert!(matches!(&StatusCode::OK, &StatusCode::OK));
}

#[test]
fn version_constants() {
    for (version, expected) in &[
        (Version::HTTP_09, "HTTP/0.9"),
        (Version::HTTP_10, "HTTP/1.0"),
        (Version::HTTP_11, "HTTP/1.1"),
        (Version::HTTP_2, "HTTP/2.0"),
        (Version::HTTP_3, "HTTP/3.0"),
    ] {
        let matched = match *version {
            Version::HTTP_09 => "HTTP/0.9",
            Version::HTTP_10 => "HTTP/1.0",
            Version::HTTP_11 => "HTTP/1.1",
            Version::HTTP_2 => "HTTP/2.0",
            Version::HTTP_3 => "HTTP/3.0",
            _ => panic!("unrecognized version"),
        };
        assert_eq!(matched, *expected);
    }

    assert!(matches!(&Version::HTTP_11, &Version::HTTP_11));
}

#[test]
fn header_name_constants() {
    for name in &["accept", "content-type", "host", "x-custom"] {
        // Standard names parsed with either casing must match the constants.
        for input in &[name.to_string(), name.to_uppercase()] {
            let header = HeaderName::from_bytes(input.as_bytes()).unwrap();
            let matched = match header {
                ACCEPT => "accept",
                CONTENT_TYPE => "content-type",
                HOST => "host",
                _ => "x-custom",
            };
            assert_eq!(matched, *name);
        }
    }

    assert!(matches!(&CONTENT_TYPE, &CONTENT_TYPE));
}
