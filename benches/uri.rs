#![feature(test)]

extern crate test;

use http::Uri;
use test::Bencher;

#[bench]
fn uri_parse_slash(b: &mut Bencher) {
    b.bytes = 1;
    b.iter(|| {
        "/".parse::<Uri>().unwrap();
    });
}

#[bench]
fn uri_parse_relative_medium(b: &mut Bencher) {
    let s = "/wp-content/uploads/2010/03/hello-kitty-darth-vader-pink.jpg";
    b.bytes = s.len() as u64;
    b.iter(|| {
        s.parse::<Uri>().unwrap();
    });
}

#[bench]
fn uri_parse_relative_query(b: &mut Bencher) {
    let s = "/wp-content/uploads/2010/03/hello-kitty-darth-vader-pink.jpg?foo={bar}|baz%13%11quux";
    b.bytes = s.len() as u64;
    b.iter(|| {
        s.parse::<Uri>().unwrap();
    });
}

#[bench]
fn uri_ord_sort(b: &mut Bencher) {
    let unordered = vec![
        "https://example.com/path?query",
        "HTTP://EXAMPLE.COM/",
        "http://example.COM/qath?query",
        "http://example.com/",
        "http://example.com/path?query",
        "file://foo/bar/framis/",
        "file://foo/bar/framiz",
        "FILE://foo/bar/framis/",
        "https://ACME.COM:80/",
        "https://acme.com:443/",
        "http://localGhost/boo/",
        "http://localhost/boo/",
        "bile://black.org/",
        "http://black.org/",
        "https://acme.com/",
        "http://acme.com/",
        "/path",
        "/?query",
        "/",
    ];

    let unordered: Vec<Uri> = unordered.iter()
        .map(|s| s.parse().expect(s))
        .collect();

    b.iter(|| {
        let mut ord = unordered.clone();
        ord.sort(); // stable
        assert_eq!(ord.first().unwrap().to_string(), "/");
        assert_eq!(ord.last().unwrap().to_string(), "http://localhost/boo/");
    });
}
