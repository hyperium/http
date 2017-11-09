use std::str::FromStr;

use super::{ErrorKind, InvalidUri, Uri, URI_CHARS};

#[test]
fn test_char_table() {
    for (i, &v) in URI_CHARS.iter().enumerate() {
        if v != 0 {
            assert_eq!(i, v as usize);
        }
    }
}

macro_rules! test_parse {
    (
        $test_name:ident,
        $str:expr,
        $alt:expr,
        $($method:ident = $value:expr,)*
    ) => (
        #[test]
        fn $test_name() {
            let uri = Uri::from_str($str).unwrap();
            $(
            assert_eq!(uri.$method(), $value, stringify!($method));
            )+
            assert_eq!(uri, *$str);
            assert_eq!(uri, uri.clone());

            const ALT: &'static [&'static str] = &$alt;

            for &alt in ALT.iter() {
                let other: Uri = alt.parse().unwrap();
                assert_eq!(uri, *alt);
                assert_eq!(uri, other);
            }
        }
    );
}

test_parse! {
    test_uri_parse_path_and_query,
    "/some/path/here?and=then&hello#and-bye",
    [],

    scheme = None,
    authority_part = None,
    path = "/some/path/here",
    query = Some("and=then&hello"),
    host = None,
}

test_parse! {
    test_uri_parse_absolute_form,
    "http://127.0.0.1:61761/chunks",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1:61761".parse().unwrap()),
    path = "/chunks",
    query = None,
    host = Some("127.0.0.1"),
    port = Some(61761),
}

test_parse! {
    test_uri_parse_absolute_form_without_path,
    "https://127.0.0.1:61761",
    ["https://127.0.0.1:61761/"],

    scheme = Some("https"),
    authority_part = Some(&"127.0.0.1:61761".parse().unwrap()),
    path = "/",
    query = None,
    port = Some(61761),
    host = Some("127.0.0.1"),
}

test_parse! {
    test_uri_parse_asterisk_form,
    "*",
    [],

    scheme = None,
    authority_part = None,
    path = "*",
    query = None,
    host = None,
}

test_parse! {
    test_uri_parse_authority_no_port,
    "localhost",
    ["LOCALHOST", "LocaLHOSt"],

    scheme = None,
    authority_part = Some(&"localhost".parse().unwrap()),
    path = "",
    query = None,
    port = None,
    host = Some("localhost"),
}

test_parse! {
    test_uri_parse_authority_form,
    "localhost:3000",
    ["localhosT:3000"],

    scheme = None,
    authority_part = Some(&"localhost:3000".parse().unwrap()),
    path = "",
    query = None,
    host = Some("localhost"),
    port = Some(3000),
}

test_parse! {
    test_uri_parse_absolute_with_default_port_http,
    "http://127.0.0.1:80",
    ["http://127.0.0.1:80/"],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1:80".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = Some(80),
}

test_parse! {
    test_uri_parse_absolute_with_default_port_https,
    "https://127.0.0.1:443",
    ["https://127.0.0.1:443/"],

    scheme = Some("https"),
    authority_part = Some(&"127.0.0.1:443".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = Some(443),
}

test_parse! {
    test_uri_parse_fragment_questionmark,
    "http://127.0.0.1/#?",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_uri_parse_path_with_terminating_questionmark,
    "http://127.0.0.1/path?",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1".parse().unwrap()),
    path = "/path",
    query = Some(""),
    port = None,
}

test_parse! {
    test_uri_parse_absolute_form_with_empty_path_and_nonempty_query,
    "http://127.0.0.1?foo=bar",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1".parse().unwrap()),
    path = "/",
    query = Some("foo=bar"),
    port = None,
}

test_parse! {
    test_uri_parse_absolute_form_with_empty_path_and_fragment_with_slash,
    "http://127.0.0.1#foo/bar",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1".parse().unwrap()),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_uri_parse_absolute_form_with_empty_path_and_fragment_with_questionmark,
    "http://127.0.0.1#foo?bar",
    [],

    scheme = Some("http"),
    authority_part = Some(&"127.0.0.1".parse().unwrap()),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_userinfo1,
    "http://a:b@127.0.0.1:1234/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"a:b@127.0.0.1:1234".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = Some(1234),
}

test_parse! {
    test_userinfo2,
    "http://a:b@127.0.0.1/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"a:b@127.0.0.1".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_userinfo3,
    "http://a@127.0.0.1/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"a@127.0.0.1".parse().unwrap()),
    host = Some("127.0.0.1"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_ipv6,
    "http://[2001:0db8:85a3:0000:0000:8a2e:0370:7334]/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"[2001:0db8:85a3:0000:0000:8a2e:0370:7334]".parse().unwrap()),
    host = Some("2001:0db8:85a3:0000:0000:8a2e:0370:7334"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_ipv6_shorthand,
    "http://[::1]/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"[::1]".parse().unwrap()),
    host = Some("::1"),
    path = "/",
    query = None,
    port = None,
}


test_parse! {
    test_ipv6_shorthand2,
    "http://[::]/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"[::]".parse().unwrap()),
    host = Some("::"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_ipv6_shorthand3,
    "http://[2001:db8::2:1]/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"[2001:db8::2:1]".parse().unwrap()),
    host = Some("2001:db8::2:1"),
    path = "/",
    query = None,
    port = None,
}

test_parse! {
    test_ipv6_with_port,
    "http://[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:8008/",
    [],

    scheme = Some("http"),
    authority_part = Some(&"[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:8008".parse().unwrap()),
    host = Some("2001:0db8:85a3:0000:0000:8a2e:0370:7334"),
    path = "/",
    query = None,
    port = Some(8008),
}

test_parse! {
    test_percentage_encoded_path,
    "/echo/abcdefgh_i-j%20/abcdefg_i-j%20478",
    [],

    scheme = None,
    authority_part = None,
    host = None,
    path = "/echo/abcdefgh_i-j%20/abcdefg_i-j%20478",
    query = None,
    port = None,
}

#[test]
fn test_uri_parse_error() {
    fn err(s: &str) {
        Uri::from_str(s).unwrap_err();
    }

    err("http://");
    err("htt:p//host");
    err("hyper.rs/");
    err("hyper.rs?key=val");
    err("?key=val");
    err("localhost/");
    err("localhost?key=val");
    err("\0");
    err("http://[::1");
    err("http://::1]");
}

#[test]
fn test_max_uri_len() {
    let mut uri = vec![];
    uri.extend(b"http://localhost/");
    uri.extend(vec![b'a'; 70 * 1024]);

    let uri = String::from_utf8(uri).unwrap();
    let res: Result<Uri, InvalidUri> = uri.parse();

    assert_eq!(res.unwrap_err().0, ErrorKind::TooLong);
}

#[test]
fn test_long_scheme() {
    let mut uri = vec![];
    uri.extend(vec![b'a'; 256]);
    uri.extend(b"://localhost/");

    let uri = String::from_utf8(uri).unwrap();
    let res: Result<Uri, InvalidUri> = uri.parse();

    assert_eq!(res.unwrap_err().0, ErrorKind::SchemeTooLong);
}

#[test]
fn test_uri_to_path_and_query() {
    let cases = vec![
        ("/", "/"),
        ("/foo?bar", "/foo?bar"),
        ("/foo?bar#nope", "/foo?bar"),
        ("http://hyper.rs", "/"),
        ("http://hyper.rs/", "/"),
        ("http://hyper.rs/path", "/path"),
        ("http://hyper.rs?query", "/?query"),
        ("*", "*"),
    ];

    for case in cases {
        let uri = Uri::from_str(case.0).unwrap();
        let s = uri.path_and_query().unwrap().to_string();

        assert_eq!(s, case.1);
    }
}
