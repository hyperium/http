# 0.1.5 (February 28, 2018)

* Add websocket handshake related header constants (#162).
* Parsing `Authority` with an empty string now returns an error (#164).
* Implement `PartialEq<u16>` for `StatusCode` (#153).
* Implement `HttpTryFrom<&Uri>` for `Uri` (#165).
* Implement `FromStr` for `Method` (#167).
* Implement `HttpTryFrom<String>` for `Uri` (#171).
* Add `into_body` fns to `Request` and `Response` (#172).
* Fix `Request::options` (#177).

# 0.1.4 (January 4, 2018)

* Add PathAndQuery::from_static (#148).
* Impl PartialOrd / PartialEq for Authority and PathAndQuery (#150).
* Add `map` fn to `Request` and `Response` (#151).

# 0.1.3 (December 11, 2017)

* Add `Scheme` associated consts for common protos.

# 0.1.2 (November 29, 2017)

* Add Uri accessor for scheme part.
* Fix Uri parsing bug (#134)

# 0.1.1 (October 9, 2017)

* Provide Uri accessors for parts (#129)
* Add Request builder helpers. (#123)
* Misc performance improvements (#126)

# 0.1.0 (September 8, 2017)

* Initial release.
