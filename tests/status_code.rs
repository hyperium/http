use http::*;
use std::convert::{TryFrom, TryInto};

#[test]
fn from_bytes() {
    for ok in &[
        "100", "101", "199", "200", "250", "299",
        "321", "399", "499", "599", "600", "999"
    ] {
        assert!(StatusCode::from_bytes(ok.as_bytes()).is_ok());
    }

    for not_ok in &[
        "-100", "-10", "", "0", "00", "000",
        "10", "40", "99", "010","099",
        "1000", "1999"
    ] {
        assert!(StatusCode::from_bytes(not_ok.as_bytes()).is_err());
    }

    let giant = Box::new([b'9'; 1*1024*1024]);
    assert!(StatusCode::from_bytes(&giant[..]).is_err());
}

#[test]
fn conversions() {
    let min = StatusCode::CONTINUE;
    assert_eq!(min.try_into(), Ok(100u16));
    assert_eq!(min.try_into(), Ok(100u32));
    assert_eq!(min.try_into(), Ok(100usize));
    assert_eq!(min.try_into(), Ok(100u64));
    assert_eq!(min.try_into(), Ok(100i32));

    let max = StatusCode::try_from(999).unwrap();
    assert_eq!(u16::from(max), 999);
    assert_eq!(u32::from(max), 999);
    assert_eq!(u64::from(max), 999);
    assert_eq!(usize::from(max), 999);
    assert_eq!(i16::from(max), 999);
    assert_eq!(i32::from(max), 999);
    assert_eq!(i64::from(max), 999);
    assert_eq!(isize::from(max), 999);
}

#[test]
fn partial_eq_ne() {
    let status = StatusCode::from_u16(200u16).unwrap();
    assert_eq!(200u16, status);
    assert_eq!(status, 200u16);

    assert_eq!(200i16, status);
    assert_eq!(status, 200i16);

    assert_eq!(200u32, status);
    assert_eq!(status, 200u32);

    assert_eq!(200u64, status);
    assert_eq!(status, 200u64);

    assert_ne!(status, 201u16);
    assert_ne!(status, 201u32);
    assert_ne!(status, 0u16);
    assert_ne!(status, -3000i16);
    assert_ne!(status, -10000i32);
}

#[test]
fn roundtrip() {
    for s in 100..1000 {
        let sstr = s.to_string();
        let status = StatusCode::from_bytes(sstr.as_bytes()).unwrap();
        assert_eq!(s, u16::from(status));
        assert_eq!(sstr, status.as_str());
    }
}

#[test]
fn is_informational() {
    assert!(status_code(100).is_informational());
    assert!(status_code(199).is_informational());

    assert!(!status_code(200).is_informational());
}

#[test]
fn is_success() {
    assert!(status_code(200).is_success());
    assert!(status_code(299).is_success());

    assert!(!status_code(199).is_success());
    assert!(!status_code(300).is_success());
}

#[test]
fn is_redirection() {
    assert!(status_code(300).is_redirection());
    assert!(status_code(399).is_redirection());

    assert!(!status_code(299).is_redirection());
    assert!(!status_code(400).is_redirection());
}

#[test]
fn is_client_error() {
    assert!(status_code(400).is_client_error());
    assert!(status_code(499).is_client_error());

    assert!(!status_code(399).is_client_error());
    assert!(!status_code(500).is_client_error());
}

#[test]
fn is_server_error() {
    assert!(status_code(500).is_server_error());
    assert!(status_code(599).is_server_error());

    assert!(!status_code(499).is_server_error());
    assert!(!status_code(600).is_server_error());
}

/// Helper method for readability
fn status_code(status_code: u16) -> StatusCode {
    StatusCode::from_u16(status_code).unwrap()
}
