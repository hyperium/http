#![feature(test)]

extern crate http;
extern crate test;


const CUSTOM: &'static [&'static str] = &[
    "x-custom-01", "x-custom-02", "x-custom-03", "x-custom-04", "x-custom-5",
    "x-custom-06", "x-custom-07", "x-custom-08", "x-custom-09", "x-custom-10",
];

mod header_map {
    use super::CUSTOM;
    use http::*;
    use http::header::*;
    use test::{self, Bencher};

    #[bench]
    fn set_get_host(b: &mut Bencher) {
        b.iter(|| {
            let mut h = HeaderMap::new();

            h.insert(HOST, HeaderValue::from("hyper.rs"));

            test::black_box(h.get(&HOST));

            h
        })
    }

    #[bench]
    fn set_get_content_length(b: &mut Bencher) {
        b.iter(|| {
            let mut h = HeaderMap::new();

            h.insert(CONTENT_LENGTH, HeaderValue::from_static("1024"));

            test::black_box(h.get(&CONTENT_LENGTH));

            h
        })
    }

    #[bench]
    fn set_10_get_1_custom(b: &mut Bencher) {
        let hdrs: Vec<HeaderName> = CUSTOM.iter()
            .map(|h| h.parse().unwrap())
            .collect();

        b.iter(|| {
            let mut h = HeaderMap::new();

            for hdr in &hdrs {
                h.insert(hdr, HeaderValue::from("foo"));
            }

            test::black_box(h.get(&CUSTOM[0]));
        })
    }

    #[bench]
    fn misc_set_11_get_many(b: &mut Bencher) {
        let hdrs: &[(&'static str, &'static str)] = &[
            ("Date", "Fri, 27 Jan 2017 23:00:00 GMT"),
            ("Content-Type", "text/html; charset=utf-8"),
            ("Transfer-Encoding", "chunked"),
            ("Connection", "keep-alive"),
            ("Set-Cookie", "__cfduid=dbdfbbe3822b61cb8750ba37d894022151485558000; expires=Sat, 27-Jan-18 23:00:00 GMT; path=/; domain=.ycombinator.com; HttpOnly"),
            ("Vary", "Accept-Encoding"),
            ("Cache-Control", "private"),
            ("X-Frame-Options", "DENY"),
            ("Strict-Transport-Security", "max-age=31556900; includeSubDomains"),
            ("Server", "cloudflare-nginx"),
            ("CF-RAY", "327fd1809f3c1baf-SEA"),
        ];

        b.iter(|| {
            let mut h = HeaderMap::new();

            for &(name, val) in hdrs.iter() {
                h.insert(name, val.to_string());
            }

            for _ in 0..10 {
                test::black_box(h.get(&CONTENT_LENGTH));
                test::black_box(h.get(&VARY));
                test::black_box(h.get("CF-RAY"));
            }
        });
    }

    #[bench]
    fn misc_set_many_std_and_custom_get_many(b: &mut Bencher) {
        let hdrs: &[(&'static str, &'static str)] = &[
            ("Date", "Fri, 27 Jan 2017 23:00:00 GMT"),
            ("Content-Type", "text/html; charset=utf-8"),
            ("Transfer-Encoding", "chunked"),
            ("Connection", "keep-alive"),
            ("Set-Cookie", "__cfduid=dbdfbbe3822b61cb8750ba37d894022151485558000; expires=Sat, 27-Jan-18 23:00:00 GMT; path=/; domain=.ycombinator.com; HttpOnly"),
            ("Vary", "Accept-Encoding"),
            ("Cache-Control", "private"),
            ("X-Frame-Options", "DENY"),
            ("Strict-Transport-Security", "max-age=31556900; includeSubDomains"),
            ("Server", "cloudflare-nginx"),
            ("CF-RAY", "327fd1809f3c1baf-SEA"),
            ("x-custom-01", "foo"),
            ("x-custom-02", "bar"),
            ("x-custom-03", "baaaz"),
            ("x-custom-04", "wat"),
            ("x-custom-05", "more custom"),
            ("x-custom-06", "boring"),
            ("x-custom-07", "omg so much typing"),
        ];

        b.iter(|| {
            let mut h = HeaderMap::new();

            for &(name, val) in hdrs.iter() {
                h.insert(name, val.to_string());
            }

            for _ in 0..10 {
                test::black_box(h.get(&CONTENT_LENGTH));
                test::black_box(h.get(&VARY));
                test::black_box(h.get("CF-RAY"));
                test::black_box(h.get("x-custom-06"));
            }
        });
    }
}

mod hash_map_default_hasher {
    use super::CUSTOM;
    use http::header::*;
    use test::{self, Bencher};
    use std::collections::HashMap;


    #[bench]
    fn set_get_host(b: &mut Bencher) {
        b.iter(|| {
            let mut h = HashMap::new();

            h.insert(HOST, HeaderValue::from("hyper.rs"));

            test::black_box(h.get(&HOST));

            h
        })
    }

    #[bench]
    fn set_get_content_length(b: &mut Bencher) {
        b.iter(|| {
            let mut h = HashMap::new();

            h.insert(CONTENT_LENGTH, HeaderValue::from_static("1024"));

            test::black_box(h.get(&CONTENT_LENGTH));

            h
        })
    }

    #[bench]
    fn set_10_get_1_custom(b: &mut Bencher) {
        let hdrs: Vec<HeaderName> = CUSTOM.iter()
            .map(|h| h.parse().unwrap())
            .collect();

        b.iter(|| {
            let mut h = HashMap::new();

            for hdr in &hdrs {
                h.insert(hdr.clone(), HeaderValue::from("foo"));
            }

            test::black_box(h.get(CUSTOM[0]));
        })
    }

    #[bench]
    fn misc_set_11_get_many(b: &mut Bencher) {
        let hdrs: &[(&'static str, &'static str)] = &[
            ("Date", "Fri, 27 Jan 2017 23:00:00 GMT"),
            ("Content-Type", "text/html; charset=utf-8"),
            ("Transfer-Encoding", "chunked"),
            ("Connection", "keep-alive"),
            ("Set-Cookie", "__cfduid=dbdfbbe3822b61cb8750ba37d894022151485558000; expires=Sat, 27-Jan-18 23:00:00 GMT; path=/; domain=.ycombinator.com; HttpOnly"),
            ("Vary", "Accept-Encoding"),
            ("Cache-Control", "private"),
            ("X-Frame-Options", "DENY"),
            ("Strict-Transport-Security", "max-age=31556900; includeSubDomains"),
            ("Server", "cloudflare-nginx"),
            ("CF-RAY", "327fd1809f3c1baf-SEA"),
        ];

        b.iter(|| {
            let mut h = HashMap::new();

            for &(name, val) in hdrs.iter() {
                let name: HeaderName = name.parse().unwrap();
                h.insert(name, val.to_string());
            }

            for _ in 0..10 {
                test::black_box(h.get(&CONTENT_LENGTH));
                test::black_box(h.get(&VARY));
                test::black_box(h.get("CF-RAY"));
            }
        });
    }

    #[bench]
    fn misc_set_many_std_and_custom_get_many(b: &mut Bencher) {
        let hdrs: &[(&'static str, &'static str)] = &[
            ("Date", "Fri, 27 Jan 2017 23:00:00 GMT"),
            ("Content-Type", "text/html; charset=utf-8"),
            ("Transfer-Encoding", "chunked"),
            ("Connection", "keep-alive"),
            ("Set-Cookie", "__cfduid=dbdfbbe3822b61cb8750ba37d894022151485558000; expires=Sat, 27-Jan-18 23:00:00 GMT; path=/; domain=.ycombinator.com; HttpOnly"),
            ("Vary", "Accept-Encoding"),
            ("Cache-Control", "private"),
            ("X-Frame-Options", "DENY"),
            ("Strict-Transport-Security", "max-age=31556900; includeSubDomains"),
            ("Server", "cloudflare-nginx"),
            ("CF-RAY", "327fd1809f3c1baf-SEA"),
            ("x-custom-01", "foo"),
            ("x-custom-02", "bar"),
            ("x-custom-03", "baaaz"),
            ("x-custom-04", "wat"),
            ("x-custom-05", "more custom"),
            ("x-custom-06", "boring"),
            ("x-custom-07", "omg so much typing"),
        ];

        b.iter(|| {
            let mut h = HashMap::new();

            for &(name, val) in hdrs.iter() {
                let name: HeaderName = name.parse().unwrap();
                h.insert(name, val.to_string());
            }

            for _ in 0..10 {
                test::black_box(h.get(&CONTENT_LENGTH));
                test::black_box(h.get(&VARY));
                test::black_box(h.get("CF-RAY"));
                test::black_box(h.get("x-custom-06"));
            }
        });
    }
}
