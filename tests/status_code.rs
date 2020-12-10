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

    let max = StatusCode::try_from(999).unwrap();
    assert_eq!(u16::from(max), 999);
}

#[test]
fn partial_eq_ne() {
    let status = StatusCode::from_u16(200u16).unwrap();
    assert_eq!(200u16, status);
    assert_eq!(status, 200u16);
    assert_ne!(status, 201u16);
    assert_ne!(status, 0u16);
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
