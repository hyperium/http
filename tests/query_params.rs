extern crate http;
use http::uri::PathAndQuery;


#[test]
fn path_and_query_param_empty() {
    let p = PathAndQuery::from_static("/path");
    assert_eq!(p.query(), None, "Query string is expected to be None");
    
    let params = p.query_params();
    assert!(params.is_empty(), "Params expected to be empty");
}

#[test]
fn path_and_query_param_single() {
    let p = PathAndQuery::from_static("/path?key=value");
    assert_eq!(p.query(), Some("key=value"), "Query string is expected to be not empty");
    
    assert!(p.query_contains_key("key"), "Query is expected to contain key 'key'");
    assert_eq!(p.query_param("key"), Some(vec!["value"]), "Key value for 'key' is expected to be {:?}", vec!["value"]);
}


#[test]
fn path_and_query_param_several() {
    let p = PathAndQuery::from_static("/path?key=value&foo=bar&boo=baz");
    assert!(p.query_contains_key("key"));
    assert_eq!(p.query_param("key"), Some(vec!["value"]));
    assert!(p.query_contains_key("foo"));
    assert_eq!(p.query_param("foo"), Some(vec!["bar"]));
    assert!(p.query_contains_key("boo"));
    assert_eq!(p.query_param("boo"), Some(vec!["baz"]));
}


#[test]
fn path_and_query_param_multi() {
    let p = PathAndQuery::from_static("/path?key=value1&key=value2");
    assert!(p.query_contains_key("key"), "Query is expected to contain key 'key'");
    assert_eq!(p.query_param("key"), Some(vec!["value1", "value2"]), "Key value for 'key' is expected to be {:?}", vec!["value1", "value2"]);
}


#[test]
fn path_and_query_param_several_with_multi() {
    let p = PathAndQuery::from_static("/path?key=value&foo=bar&boo=baz&foo=booble&key=sobeit");
    assert!(p.query_contains_key("key"));
    assert_eq!(p.query_param("key"), Some(vec!["value", "sobeit"]));
    assert!(p.query_contains_key("foo"));
    assert_eq!(p.query_param("foo"), Some(vec!["bar", "booble"]));
    assert!(p.query_contains_key("boo"));
    assert_eq!(p.query_param("boo"), Some(vec!["baz"]));
}


#[test]
fn path_and_query_param_first() {
    let p = PathAndQuery::from_static("/path?key=value1&key=value2");
    assert!(p.query_contains_key("key"), "Query is expected to contain key 'key'");
    assert_eq!(p.query_param_first("key"), Some("value1"), "Key value for 'key' is expected to be 'value1'");
}

