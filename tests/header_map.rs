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
            assert_eq!(e.first(), &"world");

            // Push another value
            e.insert("zomg");

            assert_eq!(*e.first(), "world");
            assert_eq!(*e.last(), "zomg");
        }
        _ => panic!(),
    }
}

#[test]
fn drain() {
    let mut headers = HeaderMap::new();

    // Insert a single value
    headers.set("hello", "world");

    {
        let mut iter = headers.drain();
        let (name, values) = iter.next().unwrap();
        assert_eq!(name.as_str(), "hello");

        let values: Vec<_> = values.collect();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], "world");

        assert!(iter.next().is_none());
    }

    assert!(headers.is_empty());

    // Insert two sequential values
    headers.insert("hello", "world");
    headers.set("zomg", "bar");
    headers.insert("hello", "world2");

    // Drain...
    {
        let mut iter = headers.drain();
        let (name, values) = iter.next().unwrap();
        assert_eq!(name.as_str(), "hello");

        let values: Vec<_> = values.collect();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], "world");
        assert_eq!(values[1], "world2");

        let (name, values) = iter.next().unwrap();
        assert_eq!(name.as_str(), "zomg");

        let values: Vec<_> = values.collect();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], "bar");

        assert!(iter.next().is_none());
    }
}

#[test]
fn drain_entry() {
    let mut headers = HeaderMap::new();

    headers.insert("hello", "world");
    headers.set("zomg", "foo");
    headers.insert("hello", "world2");
    headers.insert("more", "words");
    headers.insert("more", "insertions");

    // Using set
    {
        let vals: Vec<_> = headers.set("hello", "wat").unwrap().collect();
        assert_eq!(2, vals.len());
        assert_eq!(vals[0], "world");
        assert_eq!(vals[1], "world2");
    }
}

#[test]
fn eq() {
    let mut a = HeaderMap::new();
    let mut b = HeaderMap::new();

    assert_eq!(a, b);

    a.set("hello", "world");
    assert_ne!(a, b);

    b.set("hello", "world");
    assert_eq!(a, b);

    a.insert("foo", "bar");
    a.insert("foo", "baz");
    assert_ne!(a, b);

    b.insert("foo", "bar");
    assert_ne!(a, b);

    b.insert("foo", "baz");
    assert_eq!(a, b);

    a.insert("a", "a");
    a.insert("a", "b");
    b.insert("a", "b");
    b.insert("a", "a");

    assert_ne!(a, b);
}
