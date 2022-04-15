use http::*;

#[test]
fn from_bytes() {
    for ok in &[
        "100", "101", "199", "200", "250", "299", "321", "399", "499", "599", "600", "999"
    ] {
        assert!(StatusCode::from_bytes(ok.as_bytes()).is_ok());
    }

    for not_ok in &[
        "0", "00", "10", "40", "99", "000", "010", "099", "1000", "1999",
    ] {
        assert!(StatusCode::from_bytes(not_ok.as_bytes()).is_err());
    }
}

#[test]
fn equates_with_u16() {
    let status = StatusCode::from_u16(200u16).unwrap();
    assert_eq!(200u16, status);
    assert_eq!(status, 200u16);
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
    assert_eq!(true, StatusCode::from_u16(100).unwrap().is_informational());
    assert_eq!(true, StatusCode::from_u16(199).unwrap().is_informational());
    assert_eq!(false, StatusCode::from_u16(200).unwrap().is_informational());
}

#[test]
fn is_success() {
    assert_eq!(false, StatusCode::from_u16(199).unwrap().is_success());
    assert_eq!(true, StatusCode::from_u16(200).unwrap().is_success());
    assert_eq!(true, StatusCode::from_u16(299).unwrap().is_success());
    assert_eq!(false, StatusCode::from_u16(300).unwrap().is_success());
}

#[test]
fn is_redirection() {
    assert_eq!(false, StatusCode::from_u16(299).unwrap().is_redirection());
    assert_eq!(true, StatusCode::from_u16(300).unwrap().is_redirection());
    assert_eq!(true, StatusCode::from_u16(399).unwrap().is_redirection());
    assert_eq!(false, StatusCode::from_u16(400).unwrap().is_redirection());
}

#[test]
fn is_client_error() {
    assert_eq!(false, StatusCode::from_u16(399).unwrap().is_client_error());
    assert_eq!(true, StatusCode::from_u16(400).unwrap().is_client_error());
    assert_eq!(true, StatusCode::from_u16(499).unwrap().is_client_error());
    assert_eq!(false, StatusCode::from_u16(500).unwrap().is_client_error());
}

#[test]
fn is_server_error() {
    assert_eq!(false, StatusCode::from_u16(499).unwrap().is_server_error());
    assert_eq!(true, StatusCode::from_u16(500).unwrap().is_server_error());
    assert_eq!(true, StatusCode::from_u16(599).unwrap().is_server_error());
    assert_eq!(false, StatusCode::from_u16(600).unwrap().is_server_error());
}