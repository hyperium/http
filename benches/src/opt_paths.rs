use criterion::{black_box, criterion_group, criterion_main, Criterion};
use http::header::*;
use http::{HeaderValue, Uri};

static SHORT: &[u8] = b"localhost";
static LONG: &[u8] = b"Mozilla/5.0 (X11; CrOS x86_64 9592.71.0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.80 Safari/537.36";

const REL: &str = "/wp-content/uploads/2010/03/hello-kitty-darth-vader-pink.jpg";
const REL_QUERY: &str = "/wp-content/uploads/2010/03/hello-kitty-darth-vader-pink.jpg?foo=bar&baz=quux";
const ABS: &str = "https://www.example.com/wp-content/uploads/hello.jpg?foo=bar";

const STD: &[HeaderName] = &[
    HOST, CONTENT_TYPE, CONTENT_LENGTH, ACCEPT, ACCEPT_ENCODING, USER_AGENT,
    CONNECTION, CACHE_CONTROL, DATE, SERVER,
];

fn header_value(c: &mut Criterion) {
    c.bench_function("hv_from_bytes_short", |b| {
        b.iter(|| HeaderValue::from_bytes(black_box(SHORT)).unwrap())
    });
    c.bench_function("hv_from_bytes_long", |b| {
        b.iter(|| HeaderValue::from_bytes(black_box(LONG)).unwrap())
    });
    let short = HeaderValue::from_bytes(SHORT).unwrap();
    let long = HeaderValue::from_bytes(LONG).unwrap();
    c.bench_function("hv_to_str_short", |b| {
        b.iter(|| black_box(&short).to_str().unwrap())
    });
    c.bench_function("hv_to_str_long", |b| {
        b.iter(|| black_box(&long).to_str().unwrap())
    });
}

fn uri(c: &mut Criterion) {
    c.bench_function("uri_parse_relative_medium", |b| {
        b.iter(|| black_box(REL).parse::<Uri>().unwrap())
    });
    c.bench_function("uri_parse_relative_query", |b| {
        b.iter(|| black_box(REL_QUERY).parse::<Uri>().unwrap())
    });
    let rel: Uri = REL.parse().unwrap();
    let rel_query: Uri = REL_QUERY.parse().unwrap();
    let abs: Uri = ABS.parse().unwrap();
    c.bench_function("uri_to_string_relative", |b| {
        b.iter(|| black_box(&rel).to_string())
    });
    c.bench_function("uri_to_string_relative_query", |b| {
        b.iter(|| black_box(&rel_query).to_string())
    });
    c.bench_function("uri_to_string_absolute", |b| {
        b.iter(|| black_box(&abs).to_string())
    });
}

fn header_map(c: &mut Criterion) {
    c.bench_function("hm_insert_10_std", |b| {
        b.iter(|| {
            let mut m = HeaderMap::default();
            for hdr in STD {
                m.insert(hdr.clone(), "foo");
            }
            black_box(m)
        })
    });
}

criterion_group!(benches, header_value, uri, header_map);
criterion_main!(benches);
