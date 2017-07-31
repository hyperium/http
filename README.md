# HTTP

A general purpose library of common HTTP types

[![Build Status](https://travis-ci.org/carllerche/http.svg?branch=master)](https://travis-ci.org/carllerche/http)
<!-- [![Crates.io](https://img.shields.io/crates/v/http.svg?maxAge=2592000)](https://crates.io/crates/http) -->
<!-- [![Documentation](https://docs.rs/http/badge.svg)][dox] -->

More information about this crate can be found in the [crate
documentation][dox]

[dox]: https://carllerche.github.io/http

## Usage

To use `http`, first add this to your `Cargo.toml`:

```toml
[dependencies]
http = { git = 'https://github.com/carllerche/http' } # soon to be on crates.io!
```

Next, add this to your crate:

```rust
extern crate http;

use http::{Request, Response};

fn main() {
    // ...
}
```

## Examples

Create an HTTP request:

```
extern crate http;

use http::Request;

fn main() {
    let request = Request::builder()
      .uri("https://www.rust-lang.org/")
      .header("User-Agent", "awsome/1.0")
      .body(())
      .unwrap();
}
```

Create an HTTP resposne:

```
extern crate http;

use http::Response;
use http::status;

fn main() {
    let response = Response::builder()
      .status(status::MOVED_PERMANENTLY)
      .header("Location", "https://www.rust-lang.org/install.html")
      .body(())
      .unwrap();
}
```

# License

`http` is primarily distributed under the terms of both the MIT license and the
Apache License (Version 2.0), with portions covered by various BSD-like
licenses.

See LICENSE-APACHE, and LICENSE-MIT for details.
