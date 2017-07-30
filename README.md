# HTTP

A general purpose library of common HTTP types

[![Build Status](https://travis-ci.org/carllerche/http.svg?branch=master)](https://travis-ci.org/carllerche/http)
[![Crates.io](https://img.shields.io/crates/v/http.svg?maxAge=2592000)](https://crates.io/crates/http)
[![Documentation](https://docs.rs/http/badge.svg)](https://docs.rs/http)

More information about this crate can be found in the [crate
documentation](https://docs.rs/http)

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
```

## Examples

Create an HTTP request:

```
use http::Request;

let request = Request::builder()
  .uri("https://www.rust-lang.org/")
  .header("User-Agent", "awsome/1.0")
  .build()
  .unwrap();
```

Create an HTTP resposne:

```
use http::Response;
use http::status;

let response = Response::builder()
  .status(status::MOVED_PERMANENTLY)
  .header("Location", "https://www.rust-lang.org/install.html")
  .build()
  .unwrap();
```

# License

`http` is primarily distributed under the terms of both the MIT license and the
Apache License (Version 2.0), with portions covered by various BSD-like
licenses.

See LICENSE-APACHE, and LICENSE-MIT for details.
