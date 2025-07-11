#[test]
fn match_scheme() {
    let s = http::uri::Scheme::HTTP;

    match s {
        http::uri::Scheme::HTTP => (),
        http::uri::Scheme::HTTPS | _ => {
            panic!("unexpected match: {:?}", s);
        }
    }
}

#[test]
fn match_metcho() {
    let m = "GET".parse::<http::Method>().unwrap();

    match m {
        http::Method::GET => (),
        http::Method::POST | _ => {
            panic!("unexpected match: {:?}", m);
        }
    }
}

#[test]
fn match_status() {

}

#[test]
fn match_version() {
    match http::Version::default() {
        http::Version::HTTP_09 => (),
        http::Version::HTTP_10 => (),
        http::Version::HTTP_11 => (),
        http::Version::HTTP_2 => (),
        http::Version::HTTP_3 => (),
        _ => (),
    }
}
