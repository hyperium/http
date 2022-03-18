use debugger_test::debugger_test;
use http::uri::Scheme;
use http::{Request, Response, StatusCode, Uri};

#[inline(never)]
fn __break() {}

#[debugger_test(
    debugger = "cdb",
    commands = r#"
.nvlist

dx request
dx request.head
dx request.head.uri
dx request.head.headers

dx response
dx response.head
dx response.head.headers

dx uri
"#,
    expected_statements = r#"
pattern:.*\.exe \(embedded NatVis .*debugger_visualizer-0\.natvis

request          [Type: http::request::Request<str>]
    [+0x000] head             : { method=Get, uri=https://www.rust-lang.org/ } [Type: http::request::Parts]
    [+0x0e0] body             : "HELLLLOOOOO WOOOOOORLLLLDDD!" [Type: str]

request.head     : { method=Get, uri=https://www.rust-lang.org/ } [Type: http::request::Parts]
    [<Raw View>]     [Type: http::request::Parts]
    [method]         : Get [Type: http::method::Method]
    [uri]            : https://www.rust-lang.org/ [Type: http::uri::Uri]
    [version]        : Http11 [Type: http::version::Version]
    [headers]        : { len=0x3 } [Type: http::header::map::HeaderMap<http::header::value::HeaderValue>]
    [extensions]     [Type: http::extensions::Extensions]

request.head.uri : https://www.rust-lang.org/ [Type: http::uri::Uri]
    [<Raw View>]     [Type: http::uri::Uri]
    [scheme]         : Https [Type: http::uri::scheme::Scheme]
    [authority]      : www.rust-lang.org [Type: http::uri::authority::Authority]
    [path_and_query] : / [Type: http::uri::path::PathAndQuery]

request.head.headers : { len=0x3 } [Type: http::header::map::HeaderMap<http::header::value::HeaderValue>]
    [<Raw View>]     [Type: http::header::map::HeaderMap<http::header::value::HeaderValue>]
    [extra_values]   : { len=0x0 } [Type: alloc::vec::Vec<http::header::map::ExtraValue<http::header::value::HeaderValue>,alloc::alloc::Global>]
    [len]            : 0x3 [Type: unsigned __int64]
    [capacity]       : 0x6 [Type: unsigned __int64]
    [1]              : { key=UserAgent, value="my-awesome-agent/1.0" } [Type: http::header::map::Bucket<http::header::value::HeaderValue>]
    [2]              : { key=ContentLanguage, value="en_US" } [Type: http::header::map::Bucket<http::header::value::HeaderValue>]

response         [Type: http::response::Response<str>]
    [+0x000] head             : { status=404 } [Type: http::response::Parts]
    [+0x070] body             : "HELLLLOOOOO WOOOOOORLLLLDDD!" [Type: str]

response.head    : { status=404 } [Type: http::response::Parts]
    [<Raw View>]     [Type: http::response::Parts]
    [status]         : 404 [Type: http::status::StatusCode]
    [version]        : Http11 [Type: http::version::Version]
    [headers]        : { len=0x0 } [Type: http::header::map::HeaderMap<http::header::value::HeaderValue>]
    [extensions]     [Type: http::extensions::Extensions]

response.head.headers : { len=0x0 } [Type: http::header::map::HeaderMap<http::header::value::HeaderValue>]
    [<Raw View>]     [Type: http::header::map::HeaderMap<http::header::value::HeaderValue>]
    [extra_values]   : { len=0x0 } [Type: alloc::vec::Vec<http::header::map::ExtraValue<http::header::value::HeaderValue>,alloc::alloc::Global>]
    [len]            : 0x0 [Type: unsigned __int64]
    [capacity]       : 0x0 [Type: unsigned __int64]

uri              : https://www.rust-lang.org/index.html [Type: http::uri::Uri]
    [<Raw View>]     [Type: http::uri::Uri]
    [scheme]         : Https [Type: http::uri::scheme::Scheme]
    [authority]      : www.rust-lang.org [Type: http::uri::authority::Authority]
    [path_and_query] : /index.html [Type: http::uri::path::PathAndQuery]
"#
)]
fn test_debugger_visualizer() {
    let request = Request::builder()
        .uri("https://www.rust-lang.org/")
        .header(http::header::AGE, 0)
        .header(http::header::USER_AGENT, "my-awesome-agent/1.0")
        .header(http::header::CONTENT_LANGUAGE, "en_US")
        .body("HELLLLOOOOO WOOOOOORLLLLDDD!")
        .unwrap();

    assert!(request.headers().contains_key(http::header::USER_AGENT));
    assert_eq!(
        "www.rust-lang.org",
        request.uri().authority().unwrap().host()
    );

    let response = send(request).expect("http response is success");
    assert!(!response.status().is_success());

    let uri = "https://www.rust-lang.org/index.html"
        .parse::<Uri>()
        .unwrap();
    assert_eq!(uri.scheme(), Some(&Scheme::HTTPS));
    assert_eq!(uri.host(), Some("www.rust-lang.org"));
    assert_eq!(uri.path(), "/index.html");
    assert_eq!(uri.query(), None);
    __break();
}

fn send(req: Request<&str>) -> http::Result<Response<&str>> {
    if req.uri() != "/awesome-url" {
        let result = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(req.body().clone());

        return result;
    }

    let body = req.body().clone();

    let response = Response::builder().status(StatusCode::OK).body(body);

    response
}
