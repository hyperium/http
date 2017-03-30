extern crate http;

use http::*;
use http::header::*;

#[test]
fn smoke() {
    let mut headers = HeaderMap::new();

    assert!(headers.get("hello").is_none());

    let name: HeaderName = "hello".parse().unwrap();

    match headers.entry(&name) {
        Entry::Vacant(e) => {
            e.set("world");
        }
        _ => panic!(),
    }

    assert!(headers.get("hello").is_some());

    match headers.entry(&name) {
        Entry::Occupied(mut e) => {
            assert_eq!(e.first(), "world");

            // Push another value
            e.insert("zomg");

            assert_eq!(*e.first(), "world");
            assert_eq!(*e.last(), "zomg");
        }
        _ => panic!(),
    }
}
