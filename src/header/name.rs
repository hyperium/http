use super::fast_hash::{self, FastHash, FastHasher};
use byte_str::ByteStr;
use bytes::{Bytes, BytesMut};

use std::{fmt, mem};
use std::hash::{Hash, Hasher};
use std::str::FromStr;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct HeaderName {
    inner: Repr<Custom>,
}

/// Almost a full `HeaderName`
#[derive(Debug, Hash)]
pub struct HdrName<'a> {
    inner: Repr<MaybeLower<'a>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum Repr<T> {
    Standard(StandardHeader),
    Custom(T),
}

// Used to hijack the Hash impl
#[derive(Debug, Clone, Eq, PartialEq)]
struct Custom(ByteStr);

#[derive(Debug, Clone)]
struct MaybeLower<'a> {
    buf: &'a [u8],
    lower: bool,
}

#[derive(Debug)]
pub struct FromBytesError {
    _priv: (),
}

#[derive(Debug)]
pub struct FromStrError {
    _priv: (),
}

macro_rules! standard_headers {
    (
        $(
            $(#[$docs:meta])*
            ($konst:ident, $upcase:ident, $name:expr, $hash:expr);
        )+
    ) => {
        #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
        enum StandardHeader {
            $(
                $konst,
            )+
        }

        $(
            $(#[$docs])*
            pub const $upcase: HeaderName = HeaderName {
                inner: Repr::Standard(StandardHeader::$konst),
            };
        )+

        impl StandardHeader {
            fn as_str(&self) -> &'static str {
                match *self {
                    $(
                    StandardHeader::$konst => $name,
                    )+
                }
            }

            fn fast_hash(&self) -> u64 {
                match *self {
                    $(
                    StandardHeader::$konst => $hash,
                    )+
                }
            }
        }
    }
}

standard_headers! {
    /// Advertises which content types the client is able to understand.
    ///
    /// The Accept request HTTP header advertises which content types, expressed
    /// as MIME types, the client is able to understand. Using content
    /// negotiation, the server then selects one of the proposals, uses it and
    /// informs the client of its choice with the Content-Type response header.
    /// Browsers set adequate values for this header depending of the context
    /// where the request is done: when fetching a CSS stylesheet a different
    /// value is set for the request than when fetching an image, video or a
    /// script.
    (Accept, ACCEPT, "accept", 0xeff6d003e398d7a5);

    /// Advertises which character set the client is able to understand.
    ///
    /// The Accept-Charset request HTTP header advertises which character set
    /// the client is able to understand. Using content negotiation, the server
    /// then selects one of the proposals, uses it and informs the client of its
    /// choice within the Content-Type response header. Browsers usually don't
    /// set this header as the default value for each content type is usually
    /// correct and transmitting it would allow easier fingerprinting.
    ///
    /// If the server cannot serve any matching character set, it can
    /// theoretically send back a 406 (Not Acceptable) error code. But, for a
    /// better user experience, this is rarely done and the more common way is
    /// to ignore the Accept-Charset header in this case.
    (AcceptCharset, ACCEPT_CHARSET, "accept-charset", 0x1b8f06ca7ed762c1);

    /// Advertises which content encoding the client is able to understand.
    ///
    /// The Accept-Encoding request HTTP header advertises which content
    /// encoding, usually a compression algorithm, the client is able to
    /// understand. Using content negotiation, the server selects one of the
    /// proposals, uses it and informs the client of its choice with the
    /// Content-Encoding response header.
    ///
    /// Even if both the client and the server supports the same compression
    /// algorithms, the server may choose not to compress the body of a
    /// response, if the identity value is also acceptable. Two common cases
    /// lead to this:
    ///
    /// * The data to be sent is already compressed and a second compression
    /// won't lead to smaller data to be transmitted. This may the case with
    /// some image formats;
    ///
    /// * The server is overloaded and cannot afford the computational overhead
    /// induced by the compression requirement. Typically, Microsoft recommends
    /// not to compress if a server use more than 80 % of its computational
    /// power.
    ///
    /// As long as the identity value, meaning no encryption, is not explicitly
    /// forbidden, by an identity;q=0 or a *;q=0 without another explicitly set
    /// value for identity, the server must never send back a 406 Not Acceptable
    /// error.
    (AcceptEncoding, ACCEPT_ENCODING, "accept-encoding", 0xed7ad0d2d46c21bb);

    /// Advertises which languages the client is able to understand.
    ///
    /// The Accept-Language request HTTP header advertises which languages the
    /// client is able to understand, and which locale variant is preferred.
    /// Using content negotiation, the server then selects one of the proposals,
    /// uses it and informs the client of its choice with the Content-Language
    /// response header. Browsers set adequate values for this header according
    /// their user interface language and even if a user can change it, this
    /// happens rarely (and is frown upon as it leads to fingerprinting).
    ///
    /// This header is a hint to be used when the server has no way of
    /// determining the language via another way, like a specific URL, that is
    /// controlled by an explicit user decision. It is recommended that the
    /// server never overrides an explicit decision. The content of the
    /// Accept-Language is often out of the control of the user (like when
    /// traveling and using an Internet Cafe in a different country); the user
    /// may also want to visit a page in another language than the locale of
    /// their user interface.
    ///
    /// If the server cannot serve any matching language, it can theoretically
    /// send back a 406 (Not Acceptable) error code. But, for a better user
    /// experience, this is rarely done and more common way is to ignore the
    /// Accept-Language header in this case.
    (AcceptLanguage, ACCEPT_LANGUAGE, "accept-language", 0xac19d32c76975414);

    /// Advertises which patch formats the server is able to understand.
    ///
    /// Accept-Patch should appear in the OPTIONS response for any resource that
    /// supports the use of the PATCH method. The presence of the
    /// Accept-Patch header in response to any method is an implicit indication
    /// that PATCH is allowed on the resource identified by the URI. The
    /// presence of a specific patch document format in this header indicates
    /// that that specific format is allowed on the resource identified by the
    /// URI.
    (AcceptPatch, ACCEPT_PATCH, "accept-patch", 0x6806ce9fd6365e43);

    /// Marker used by the server to advertise partial request support.
    ///
    /// The Accept-Ranges response HTTP header is a marker used by the server to
    /// advertise its support of partial requests. The value of this field
    /// indicates the unit that can be used to define a range.
    ///
    /// In presence of an Accept-Ranges header, the browser may try to resume an
    /// interrupted download, rather than to start it from the start again.
    (AcceptRanges, ACCEPT_RANGES, "accept-ranges", 0x8c091f75208a4f87);

    /// Preflight response indicating if the response to the request can be
    /// exposed to the page.
    ///
    /// The Access-Control-Allow-Credentials response header indicates whether
    /// or not the response to the request can be exposed to the page. It can be
    /// exposed when the true value is returned; it can't in other cases.
    ///
    /// Credentials are cookies, authorization headers or TLS client
    /// certificates.
    ///
    /// When used as part of a response to a preflight request, this indicates
    /// whether or not the actual request can be made using credentials. Note
    /// that simple GET requests are not preflighted, and so if a request is
    /// made for a resource with credentials, if this header is not returned
    /// with the resource, the response is ignored by the browser and not
    /// returned to web content.
    ///
    /// The Access-Control-Allow-Credentials header works in conjunction with
    /// the XMLHttpRequest.withCredentials property or with the credentials
    /// option in the Request() constructor of the Fetch API. Credentials must
    /// be set on both sides (the Access-Control-Allow-Credentials header and in
    /// the XHR or Fetch request) in order for the CORS request with credentials
    /// to succeed.
    (AccessControlAllowCredentials, ACCESS_CONTROL_ALLOW_CREDENTIALS, "access-control-allow-credentials", 0x123e04e1da30623d);

    /// Preflight response indicating permitted HTTP headers.
    ///
    /// The Access-Control-Allow-Headers response header is used in response to
    /// a preflight request to indicate which HTTP headers will be available via
    /// Access-Control-Expose-Headers when making the actual request.
    ///
    /// The simple headers, Accept, Accept-Language, Content-Language,
    /// Content-Type (but only with a MIME type of its parsed value (ignoring
    /// parameters) of either application/x-www-form-urlencoded,
    /// multipart/form-data, or text/plain), are always available and don't need
    /// to be listed by this header.
    ///
    /// This header is required if the request has an
    /// Access-Control-Request-Headers header.
    (AccessControlAllowHeaders, ACCESS_CONTROL_ALLOW_HEADERS, "access-control-allow-headers", 0x2efdf4c8a2f7a7b3);

    /// Preflight header response indicating permitted access methods.
    ///
    /// The Access-Control-Allow-Methods response header specifies the method or
    /// methods allowed when accessing the resource in response to a preflight
    /// request.
    (AccessControlAllowMethods, ACCESS_CONTROL_ALLOW_METHODS, "access-control-allow-methods", 0x5a06b0abdb43b934);

    /// Indicates whether the response can be shared with resources with the
    /// given origin.
    (AccessControlAllowOrigin, ACCESS_CONTROL_ALLOW_ORIGIN, "access-control-allow-origin", 0xe5c0399a935db583);

    /// Indicates which headers can be exposed as part of the response by
    /// listing their names.
    (AccessControlExposeHeaders, ACCESS_CONTROL_EXPOSE_HEADERS, "access-control-expose-headers", 0x1da428e21e5fe3fb);

    /// Indicates how long the results of a preflight request can be cached.
    (AccessControlMaxAge, ACCESS_CONTROL_MAX_AGE, "access-control-max-age", 0x5aa6caa3a3a0a341);

    /// Informs the server which HTTP headers will be used when an actual
    /// request is made.
    (AccessControlRequestHeaders, ACCESS_CONTROL_REQUEST_HEADERS, "access-control-request-headers", 0x65df4f4cd10f7086);

    /// Informs the server know which HTTP method will be used when the actual
    /// request is made.
    (AccessControlRequestMethod, ACCESS_CONTROL_REQUEST_METHOD, "access-control-request-method", 0x74a79e52fc0965e9);

    /// Indicates the time in seconds the object has been in a proxy cache.
    ///
    /// The Age header is usually close to zero. If it is Age: 0, it was
    /// probably just fetched from the origin server; otherwise It is usually
    /// calculated as a difference between the proxy's current date and the Date
    /// general header included in the HTTP response.
    (Age, AGE, "age", 0x6494b76e2a92f7ce);

    /// Lists the set of methods support by a resource.
    ///
    /// This header must be sent if the server responds with a 405 Method Not
    /// Allowed status code to indicate which request methods can be used. An
    /// empty Allow header indicates that the resource allows no request
    /// methods, which might occur temporarily for a given resource, for
    /// example.
    (Allow, ALLOW, "allow", 0xcc2e6e57b0564dc0);

    /// Advertises the availability of alternate services to clients.
    (AltSvc, ALT_SVC, "alt-svc", 0x50f2feb089c778b7);

    /// Contains the credentials to authenticate a user agent with a server.
    ///
    /// Usually this header is included after the server has responded with a
    /// 401 Unauthorized status and the WWW-Authenticate header.
    (Authorization, AUTHORIZATION, "authorization", 0xe9dbd53eddc9410);

    /// Specifies directives for caching mechanisms in both requests and
    /// responses.
    ///
    /// Caching directives are unidirectional, meaning that a given directive in
    /// a request is not implying that the same directive is to be given in the
    /// response.
    (CacheControl, CACHE_CONTROL, "cache-control", 0x82488f691fdbe3dd);

    /// Controls whether or not the network connection stays open after the
    /// current transaction finishes.
    ///
    /// If the value sent is keep-alive, the connection is persistent and not
    /// closed, allowing for subsequent requests to the same server to be done.
    ///
    /// Except for the standard hop-by-hop headers (Keep-Alive,
    /// Transfer-Encoding, TE, Connection, Trailer, Upgrade, Proxy-Authorization
    /// and Proxy-Authenticate), any hop-by-hop headers used by the message must
    /// be listed in the Connection header, so that the first proxy knows he has
    /// to consume them and not to forward them further. Standard hop-by-hop
    /// headers can be listed too (it is often the case of Keep-Alive, but this
    /// is not mandatory.
    (Connection, CONNECTION, "connection", 0xa57ad5cdc82cb4d2);

    /// Indicates if the content is expected to be displayed inline.
    ///
    /// In a regular HTTP response, the Content-Disposition response header is a
    /// header indicating if the content is expected to be displayed inline in
    /// the browser, that is, as a Web page or as part of a Web page, or as an
    /// attachment, that is downloaded and saved locally.
    ///
    /// In a multipart/form-data body, the HTTP Content-Disposition general
    /// header is a header that can be used on the subpart of a multipart body
    /// to give information about the field it applies to. The subpart is
    /// delimited by the boundary defined in the Content-Type header. Used on
    /// the body itself, Content-Disposition has no effect.
    ///
    /// The Content-Disposition header is defined in the larger context of MIME
    /// messages for e-mail, but only a subset of the possible parameters apply
    /// to HTTP forms and POST requests. Only the value form-data, as well as
    /// the optional directive name and filename, can be used in the HTTP
    /// context.
    (ContentDisposition, CONTENT_DISPOSITION, "content-disposition", 0x35b4da4ba0850266);

    /// Used to compress the media-type.
    ///
    /// When present, its value indicates what additional content encoding has
    /// been applied to the entity-body. It lets the client know, how to decode
    /// in order to obtain the media-type referenced by the Content-Type header.
    ///
    /// It is recommended to compress data as much as possible and therefore to
    /// use this field, but some types of resources, like jpeg images, are
    /// already compressed.  Sometimes using additional compression doesn't
    /// reduce payload size and can even make the payload longer.
    (ContentEncoding, CONTENT_ENCODING, "content-encoding", 0xbdf222ba151247c);

    /// Used to describe the languages indtended for the audience.
    ///
    /// This header allows a user to differentiate according to the users' own
    /// preferred language. For example, if "Content-Language: de-DE" is set, it
    /// says that the document is intended for German language speakers
    /// (however, it doesn't indicate the document is written in German. For
    /// example, it might be written in English as part of a language course for
    /// German speakers).
    ///
    /// If no Content-Language is specified, the default is that the content is
    /// intended for all language audiences. Multiple language tags are also
    /// possible, as well as applying the Content-Language header to various
    /// media types and not only to textual documents.
    (ContentLanguage, CONTENT_LANGUAGE, "content-language", 0x1576ec21205e426);

    /// Indicates the size fo the entity-body.
    ///
    /// The header value must be a decimal indicating the number of octets sent
    /// to the recipient.
    (ContentLength, CONTENT_LENGTH, "content-length", 0xe1c9fab5479e2674);

    /// Indicates an alternate location for the returned data.
    ///
    /// The principal use case is to indicate the URL of the resource
    /// transmitted as the result of content negotiation.
    ///
    /// Location and Content-Location are different: Location indicates the
    /// target of a redirection (or the URL of a newly created document), while
    /// Content-Location indicates the direct URL to use to access the resource,
    /// without the need of further content negotiation. Location is a header
    /// associated with the response, while Content-Location is associated with
    /// the entity returned.
    (ContentLocation, CONTENT_LOCATION, "content-location", 0x7c481a15bbaad44a);

    /// Contains the MD5 digest of the entity-body.
    ///
    /// The Content-MD5 entity-header field, as defined in RFC 1864 [23], is an
    /// MD5 digest of the entity-body for the purpose of providing an end-to-end
    /// message integrity check (MIC) of the entity-body. (Note: a MIC is good
    /// for detecting accidental modification of the entity-body in transit, but
    /// is not proof against malicious attacks.)
    (ContentMd5, CONTENT_MD5, "content-md5", 0x545e3b9a690da374);

    /// Indicates where in a full body message a partial message belongs.
    (ContentRange, CONTENT_RANGE, "content-range", 0x503111c14f890f08);

    /// Allows controlling resources the user agent is allowed to load for a
    /// given page.
    ///
    /// With a few exceptions, policies mostly involve specifying server origins
    /// and script endpoints. This helps guard against cross-site scripting
    /// attacks (XSS).
    (ContentSecurityPolicy, CONTENT_SECURITY_POLICY, "content-security-policy", 0x9eac9326e92e4b02);

    /// Allows experimenting with policies by monitoring their effects.
    ///
    /// The HTTP Content-Security-Policy-Report-Only response header allows web
    /// developers to experiment with policies by monitoring (but not enforcing)
    /// their effects. These violation reports consist of JSON documents sent
    /// via an HTTP POST request to the specified URI.
    (ContentSecurityPolicyReportOnly, CONTENT_SECURITY_POLICY_REPORT_ONLY, "content-security-policy-report-only", 0xe1f05b97ef837748);

    /// Used to indicate the media type of the resource.
    ///
    /// In responses, a Content-Type header tells the client what the content
    /// type of the returned content actually is. Browsers will do MIME sniffing
    /// in some cases and will not necessarily follow the value of this header;
    /// to prevent this behavior, the header X-Content-Type-Options can be set
    /// to nosniff.
    ///
    /// In requests, (such as POST or PUT), the client tells the server what
    /// type of data is actually sent.
    (ContentType, CONTENT_TYPE, "content-type", 0xb47822143f2eb82a);

    /// Contains stored HTTP cookies previously sent by the server with the
    /// Set-Cookie header.
    ///
    /// The Cookie header might be omitted entirely, if the privacy setting of
    /// the browser are set to block them, for example.
    (Cookie, COOKIE, "cookie", 0x1a309b816fba6489);

    /// Indicates the client's tracking preference.
    ///
    /// This header lets users indicate whether they would prefer privacy rather
    /// than personalized content.
    (Dnt, DNT, "dnt", 0x4efa5e93002b6162);

    /// Contains the date and time at which the message was originated.
    (Date, DATE, "date", 0xa6420b1528a9c034);

    /// Identifier for a specific version of a resource.
    ///
    /// This header allows caches to be more efficient, and saves bandwidth, as
    /// a web server does not need to send a full response if the content has
    /// not changed. On the other side, if the content has changed, etags are
    /// useful to help prevent simultaneous updates of a resource from
    /// overwriting each other ("mid-air collisions").
    ///
    /// If the resource at a given URL changes, a new Etag value must be
    /// generated. Etags are therefore similar to fingerprints and might also be
    /// used for tracking purposes by some servers. A comparison of them allows
    /// to quickly determine whether two representations of a resource are the
    /// same, but they might also be set to persist indefinitely by a tracking
    /// server.
    (Etag, ETAG, "etag", 0x2d471eabb173cbbd);

    /// Indicates expectations that need to be fulfilled by the server in order
    /// to properly handle the request.
    ///
    /// The only expectation defined in the specification is Expect:
    /// 100-continue, to which the server shall respond with:
    ///
    /// * 100 if the information contained in the header is sufficient to cause
    /// an immediate success,
    ///
    /// * 417 (Expectation Failed) if it cannot meet the expectation; or any
    /// other 4xx status otherwise.
    ///
    /// For example, the server may reject a request if its Content-Length is
    /// too large.
    ///
    /// No common browsers send the Expect header, but some other clients such
    /// as cURL do so by default.
    (Expect, EXPECT, "expect", 0xa3fdf9d58b60c082);

    /// Contains the date/time after which the response is considered stale.
    ///
    /// Invalid dates, like the value 0, represent a date in the past and mean
    /// that the resource is already expired.
    ///
    /// If there is a Cache-Control header with the "max-age" or "s-max-age"
    /// directive in the response, the Expires header is ignored.
    (Expires, EXPIRES, "expires", 0xf03cdefb06dee481);

    /// Contains information from the client-facing side of proxy servers that
    /// is altered or lost when a proxy is involved in the path of the request.
    ///
    /// The alternative and de-facto standard versions of this header are the
    /// X-Forwarded-For, X-Forwarded-Host and X-Forwarded-Proto headers.
    ///
    /// This header is used for debugging, statistics, and generating
    /// location-dependent content and by design it exposes privacy sensitive
    /// information, such as the IP address of the client. Therefore the user's
    /// privacy must be kept in mind when deploying this header.
    (Forwarded, FORWARDED, "forwarded", 0xc1c7723a74dd94cf);

    /// Contains an Internet email address for a human user who controls the
    /// requesting user agent.
    ///
    /// If you are running a robotic user agent (e.g. a crawler), the From
    /// header should be sent, so you can be contacted if problems occur on
    /// servers, such as if the robot is sending excessive, unwanted, or invalid
    /// requests.
    (From, FROM, "from", 0x52b86bd20aff06b5);

    /// Specifies the domain name of the server and (optionally) the TCP port
    /// number on which the server is listening.
    ///
    /// If no port is given, the default port for the service requested (e.g.,
    /// "80" for an HTTP URL) is implied.
    ///
    /// A Host header field must be sent in all HTTP/1.1 request messages. A 400
    /// (Bad Request) status code will be sent to any HTTP/1.1 request message
    /// that lacks a Host header field or contains more than one.
    (Host, HOST, "host", 0xdafeb1b516b284f5);

    /// Makes a request conditional based on the E-Tag.
    ///
    /// For GET and HEAD methods, the server will send back the requested
    /// resource only if it matches one of the listed ETags. For PUT and other
    /// non-safe methods, it will only upload the resource in this case.
    ///
    /// The comparison with the stored ETag uses the strong comparison
    /// algorithm, meaning two files are considered identical byte to byte only.
    /// This is weakened when the  W/ prefix is used in front of the ETag.
    ///
    /// There are two common use cases:
    ///
    /// * For GET and HEAD methods, used in combination with an Range header, it
    /// can guarantee that the new ranges requested comes from the same resource
    /// than the previous one. If it doesn't match, then a 416 (Range Not
    /// Satisfiable) response is returned.
    ///
    /// * For other methods, and in particular for PUT, If-Match can be used to
    /// prevent the lost update problem. It can check if the modification of a
    /// resource that the user wants to upload will not override another change
    /// that has been done since the original resource was fetched. If the
    /// request cannot be fulfilled, the 412 (Precondition Failed) response is
    /// returned.
    (IfMatch, IF_MATCH, "if-match", 0x2005dc4bb60b9fb9);

    /// Makes a request conditional based on the modification date.
    ///
    /// The If-Modified-Since request HTTP header makes the request conditional:
    /// the server will send back the requested resource, with a 200 status,
    /// only if it has been last modified after the given date. If the request
    /// has not been modified since, the response will be a 304 without any
    /// body; the Last-Modified header will contain the date of last
    /// modification. Unlike If-Unmodified-Since, If-Modified-Since can only be
    /// used with a GET or HEAD.
    ///
    /// When used in combination with If-None-Match, it is ignored, unless the
    /// server doesn't support If-None-Match.
    ///
    /// The most common use case is to update a cached entity that has no
    /// associated ETag.
    (IfModifiedSince, IF_MODIFIED_SINCE, "if-modified-since", 0xac2f9d07bb4afb54);

    /// Makes a request conditional based on the E-Tag.
    ///
    /// The If-None-Match HTTP request header makes the request conditional. For
    /// GET and HEAD methods, the server will send back the requested resource,
    /// with a 200 status, only if it doesn't have an ETag matching the given
    /// ones. For other methods, the request will be processed only if the
    /// eventually existing resource's ETag doesn't match any of the values
    /// listed.
    ///
    /// When the condition fails for GET and HEAD methods, then the server must
    /// return HTTP status code 304 (Not Modified). For methods that apply
    /// server-side changes, the status code 412 (Precondition Failed) is used.
    /// Note that the server generating a 304 response MUST generate any of the
    /// following header fields that would have been sent in a 200 (OK) response
    /// to the same request: Cache-Control, Content-Location, Date, ETag,
    /// Expires, and Vary.
    ///
    /// The comparison with the stored ETag uses the weak comparison algorithm,
    /// meaning two files are considered identical not only if they are
    /// identical byte to byte, but if the content is equivalent. For example,
    /// two pages that would differ only by the date of generation in the footer
    /// would be considered as identical.
    ///
    /// When used in combination with If-Modified-Since, it has precedence (if
    /// the server supports it).
    ///
    /// There are two common use cases:
    ///
    /// * For `GET` and `HEAD` methods, to update a cached entity that has an associated ETag.
    /// * For other methods, and in particular for `PUT`, `If-None-Match` used with
    /// the `*` value can be used to save a file not known to exist,
    /// guaranteeing that another upload didn't happen before, losing the data
    /// of the previous put; this problems is the variation of the lost update
    /// problem.
    (IfNoneMatch, IF_NONE_MATCH, "if-none-match", 0xa0fd96bc4180454f);

    /// Makes a request conditional based on range.
    ///
    /// The If-Range HTTP request header makes a range request conditional: if
    /// the condition is fulfilled, the range request will be issued and the
    /// server sends back a 206 Partial Content answer with the appropriate
    /// body. If the condition is not fulfilled, the full resource is sent back,
    /// with a 200 OK status.
    ///
    /// This header can be used either with a Last-Modified validator, or with
    /// an ETag, but not with both.
    ///
    /// The most common use case is to resume a download, to guarantee that the
    /// stored resource has not been modified since the last fragment has been
    /// received.
    (IfRange, IF_RANGE, "if-range", 0x5f487c7807cff9f7);

    /// Makes the request conditional based on the last modification date.
    ///
    /// The If-Unmodified-Since request HTTP header makes the request
    /// conditional: the server will send back the requested resource, or accept
    /// it in the case of a POST or another non-safe method, only if it has not
    /// been last modified after the given date. If the request has been
    /// modified after the given date, the response will be a 412 (Precondition
    /// Failed) error.
    ///
    /// There are two common use cases:
    ///
    /// * In conjunction non-safe methods, like POST, it can be used to
    /// implement an optimistic concurrency control, like done by some wikis:
    /// editions are rejected if the stored document has been modified since the
    /// original has been retrieved.
    ///
    /// * In conjunction with a range request with a If-Range header, it can be
    /// used to ensure that the new fragment requested comes from an unmodified
    /// document.
    (IfUnmodifiedSince, IF_UNMODIFIED_SINCE, "if-unmodified-since", 0x9ab2560633ff2a63);

    /// Content-Types that are acceptable for the response.
    (LastModified, LAST_MODIFIED, "last-modified", 0xa6fa11139304ac0e);

    /// Hint about how the connection and may be used to set a timeout and a
    /// maximum amount of requests.
    (KeepAlive, KEEP_ALIVE, "keep-alive", 0xcea10c567bd9e858);

    /// Allows the server to point an interested client to another resource
    /// containing metadata about the requested resource.
    (Link, LINK, "link", 0x6aa9257b3dd5c28e);

    /// Indicates the URL to redirect a page to.
    ///
    /// The Location response header indicates the URL to redirect a page to. It
    /// only provides a meaning when served with a 3xx status response.
    ///
    /// The HTTP method used to make the new request to fetch the page pointed
    /// to by Location depends of the original method and of the kind of
    /// redirection:
    ///
    /// * If 303 (See Also) responses always lead to the use of a GET method,
    /// 307 (Temporary Redirect) and 308 (Permanent Redirect) don't change the
    /// method used in the original request;
    ///
    /// * 301 (Permanent Redirect) and 302 (Found) doesn't change the method
    /// most of the time, though older user-agents may (so you basically don't
    /// know).
    ///
    /// All responses with one of these status codes send a Location header.
    ///
    /// Beside redirect response, messages with 201 (Created) status also
    /// include the Location header. It indicates the URL to the newly created
    /// resource.
    ///
    /// Location and Content-Location are different: Location indicates the
    /// target of a redirection (or the URL of a newly created resource), while
    /// Content-Location indicates the direct URL to use to access the resource
    /// when content negotiation happened, without the need of further content
    /// negotiation. Location is a header associated with the response, while
    /// Content-Location is associated with the entity returned.
    (Location, LOCATION, "location", 0xf6497f8f13049e31);

    /// Indicates the max number of intermediaries the request should be sent
    /// through.
    (MaxForwards, MAX_FORWARDS, "max-forwards", 0x97e2a38720281478);

    /// Indicates where a fetch originates from.
    ///
    /// It doesn't include any path information, but only the server name. It is
    /// sent with CORS requests, as well as with POST requests. It is similar to
    /// the Referer header, but, unlike this header, it doesn't disclose the
    /// whole path.
    (Origin, ORIGIN, "origin", 0x96e33a9e88a71ead);

    /// HTTP/1.0 header usually used for backwards compatibility.
    ///
    /// The Pragma HTTP/1.0 general header is an implementation-specific header
    /// that may have various effects along the request-response chain. It is
    /// used for backwards compatibility with HTTP/1.0 caches where the
    /// Cache-Control HTTP/1.1 header is not yet present.
    (Pragma, PRAGMA, "pragma", 0x84d678706a701186);

    /// Defines the authentication method that should be used to gain access to
    /// a proxy.
    ///
    /// Unlike `www-authenticate`, the `proxy-authenticate` header field applies
    /// only to the next outbound client on the response chain. This is because
    /// only the client that chose a given proxy is likely to have the
    /// credentials necessary for authentication. However, when multiple proxies
    /// are used within the same administrative domain, such as office and
    /// regional caching proxies within a large corporate network, it is common
    /// for credentials to be generated by the user agent and passed through the
    /// hierarchy until consumed. Hence, in such a configuration, it will appear
    /// as if Proxy-Authenticate is being forwarded because each proxy will send
    /// the same challenge set.
    ///
    /// The `proxy-authenticate` header is sent along with a `407 Proxy
    /// Authentication Required`.
    (ProxyAuthenticate, PROXY_AUTHENTICATE, "proxy-authenticate", 0x8c32dbc4f461112a);

    /// Contains the credentials to authenticate a user agent to a proxy server.
    ///
    /// This header is usually included after the server has responded with a
    /// 407 Proxy Authentication Required status and the Proxy-Authenticate
    /// header.
    (ProxyAuthorization, PROXY_AUTHORIZATION, "proxy-authorization", 0x9b6ea5b97d174d00);

    /// Associates a specific cryptographic public key with a certain server.
    ///
    /// This decreases the risk of MITM attacks with forged certificates. If one
    /// or several keys are pinned and none of them are used by the server, the
    /// browser will not accept the response as legitimate, and will not display
    /// it.
    (PublicKeyPins, PUBLIC_KEY_PINS, "public-key-pins", 0xe30b4982730c8fd);

    /// Sends reports of pinning violation to the report-uri specified in the
    /// header.
    ///
    /// Unlike `Public-Key-Pins`, this header still allows browsers to connect
    /// to the server if the pinning is violated.
    (PublicKeyPinsReportOnly, PUBLIC_KEY_PINS_REPORT_ONLY, "public-key-pins-report-only", 0xa49682b0f8b52e34);

    /// Indicates the part of a document that the server should return.
    ///
    /// Several parts can be requested with one Range header at once, and the
    /// server may send back these ranges in a multipart document. If the server
    /// sends back ranges, it uses the 206 Partial Content for the response. If
    /// the ranges are invalid, the server returns the 416 Range Not Satisfiable
    /// error. The server can also ignore the Range header and return the whole
    /// document with a 200 status code.
    (Range, RANGE, "range", 0x48532552a588c75b);

    /// Contains the address of the previous web page from which a link to the
    /// currently requested page was followed.
    ///
    /// The Referer header allows servers to identify where people are visiting
    /// them from and may use that data for analytics, logging, or optimized
    /// caching, for example.
    (Referer, REFERER, "referer", 0x78f4ab93831ad71d);

    /// Governs which referrer information should be included with requests
    /// made.
    (ReferrerPolicy, REFERRER_POLICY, "referrer-policy", 0xdb580524af0c6629);

    /// Informs the web browser that the current page or frame should be
    /// refreshed.
    (Refresh, REFRESH, "refresh", 0x6a8afb42c7c229ae);

    /// The Retry-After response HTTP header indicates how long the user agent
    /// should wait before making a follow-up request. There are two main cases
    /// this header is used:
    ///
    /// * When sent with a 503 (Service Unavailable) response, it indicates how
    /// long the service is expected to be unavailable.
    ///
    /// * When sent with a redirect response, such as 301 (Moved Permanently),
    /// it indicates the minimum time that the user agent is asked to wait
    /// before issuing the redirected request.
    (RetryAfter, RETRY_AFTER, "retry-after", 0x88f9348ca93a174f);

    /// Contains information about the software used by the origin server to
    /// handle the request.
    ///
    /// Overly long and detailed Server values should be avoided as they
    /// potentially reveal internal implementation details that might make it
    /// (slightly) easier for attackers to find and exploit known security
    /// holes.
    (Server, SERVER, "server", 0x7078d0b96973532);

    /// Used to send cookies from the server to the user agent.
    (SetCookie, SET_COOKIE, "set-cookie", 0x25a75d77c4b7238f);

    /// Tells the client to communicate with HTTPS instead of using HTTP.
    (StrictTransportSecurity, STRICT_TRANSPORT_SECURITY, "strict-transport-security", 0x2bd3d4ac07de9fd);

    /// Informs the server of transfer encodings willing to be accepted as part
    /// of the response.
    ///
    /// See also the Transfer-Encoding response header for more details on
    /// transfer encodings. Note that chunked is always acceptable for HTTP/1.1
    /// recipients and you that don't have to specify "chunked" using the TE
    /// header. However, it is useful for setting if the client is accepting
    /// trailer fields in a chunked transfer coding using the "trailers" value.
    (Te, TE, "te", 0xa9d03e0efcf03c6e);

    /// Indicates the tracking status that applied to the corresponding request.
    (Tk, TK, "tk", 0x727e308d7f89cf2b);

    /// Allows the sender to include additional fields at the end of chunked
    /// messages.
    (Trailer, TRAILER, "trailer", 0xb64ea43b3b70f7fb);

    /// Specifies the form of encoding used to safely transfer the entity to the
    /// client.
    ///
    /// `transfer-encoding` is a hop-by-hop header, that is applying to a
    /// message between two nodes, not to a resource itself. Each segment of a
    /// multi-node connection can use different `transfer-encoding` values. If
    /// you want to compress data over the whole connection, use the end-to-end
    /// header `content-encoding` header instead.
    ///
    /// When present on a response to a `HEAD` request that has no body, it
    /// indicates the value that would have applied to the corresponding `GET`
    /// message.
    (TransferEncoding, TRANSFER_ENCODING, "transfer-encoding", 0xae18377791e15069);

    /// A response to the client's tracking preference.
    ///
    /// A tracking status value (TSV) is a single character response to the
    /// user's tracking preference with regard to data collected via the
    /// designated resource. For a site-wide tracking status resource, the
    /// designated resource is any resource on the same origin server. For a Tk
    /// response header field, the target resource of the corresponding request
    /// is the designated resource, and remains so for any subsequent
    /// request-specific tracking status resource referred to by the Tk field
    /// value.
    (Tsv, TSV, "tsv", 0x4e9c95b87f3cc85d);

    /// Contains a string that allows identifying the requesting client's
    /// software.
    (UserAgent, USER_AGENT, "user-agent", 0x220348d64524c3fe);

    /// Used as part of the exchange to upgrade the protocol.
    (Upgrade, UPGRADE, "upgrade", 0xe67ca9064a838479);

    /// Sends a signal to the server expressing the client’s preference for an
    /// encrypted and authenticated response.
    (UpgradeInsecureRequests, UPGRADE_INSECURE_REQUESTS, "upgrade-insecure-requests", 0x5c423de2b362db36);

    /// Determines how to match future requests with cached responses.
    ///
    /// The `vary` HTTP response header determines how to match future request
    /// headers to decide whether a cached response can be used rather than
    /// requesting a fresh one from the origin server. It is used by the server
    /// to indicate which headers it used when selecting a representation of a
    /// resource in a content negotiation algorithm.
    ///
    /// The `vary` header should be set on a 304 Not Modified response exactly
    /// like it would have been set on an equivalent 200 OK response.
    (Vary, VARY, "vary", 0x4e67b46bf773816b);

    /// Added by proxies to track routing.
    ///
    /// The `via` general header is added by proxies, both forward and reverse
    /// proxies, and can appear in the request headers and the response headers.
    /// It is used for tracking message forwards, avoiding request loops, and
    /// identifying the protocol capabilities of senders along the
    /// request/response chain.
    (Via, VIA, "via", 0xafa70b58adb45b5f);

    /// General HTTP header contains information about possible problems with
    /// the status of the message.
    ///
    /// More than one `warning` header may appear in a response. Warning header
    /// fields can in general be applied to any message, however some warn-codes
    /// are specific to caches and can only be applied to response messages.
    (Warning, WARNING, "warning", 0x916614ef8bbad3d3);

    /// Defines the authentication method that should be used to gain access to
    /// a resource.
    (WwwAuthenticate, WWW_AUTHENTICATE, "www-authenticate", 0x2bf5bbe2274d4b74);

    /// Marker used by the server to indicate that the MIME types advertised in
    /// the `content-type` headers should not be changed and be followed.
    ///
    /// This allows to opt-out of MIME type sniffing, or, in other words, it is
    /// a way to say that the webmasters knew what they were doing.
    ///
    /// This header was introduced by Microsoft in IE 8 as a way for webmasters
    /// to block content sniffing that was happening and could transform
    /// non-executable MIME types into executable MIME types. Since then, other
    /// browsers have introduced it, even if their MIME sniffing algorithms were
    /// less aggressive.
    ///
    /// Site security testers usually expect this header to be set.
    (XContentTypeOptions, X_CONTENT_TYPE_OPTIONS, "x-content-type-options", 0x424b1bb449a9c2ea);

    /// Controls DNS prefetching.
    ///
    /// The `x-dns-prefetch-control` HTTP response header controls DNS
    /// prefetching, a feature by which browsers proactively perform domain name
    /// resolution on both links that the user may choose to follow as well as
    /// URLs for items referenced by the document, including images, CSS,
    /// JavaScript, and so forth.
    ///
    /// This prefetching is performed in the background, so that the DNS is
    /// likely to have been resolved by the time the referenced items are
    /// needed. This reduces latency when the user clicks a link.
    (XDnsPrefetchControl, X_DNS_PREFETCH_CONTROL, "x-dns-prefetch-control", 0xf8e75a6bd87b8e47);

    /// Indicates whether or not a browser should be allowed to render a page in
    /// a frame.
    ///
    /// Sites can use this to avoid clickjacking attacks, by ensuring that their
    /// content is not embedded into other sites.
    ///
    /// The added security is only provided if the user accessing the document
    /// is using a browser supporting `x-frame-options`.
    (XFrameOptions, X_FRAME_OPTIONS, "x-frame-options", 0x991f40e34fb35c26);

    /// Stop pages from loading when an XSS attack is detected.
    ///
    /// The HTTP X-XSS-Protection response header is a feature of Internet
    /// Explorer, Chrome and Safari that stops pages from loading when they
    /// detect reflected cross-site scripting (XSS) attacks. Although these
    /// protections are largely unnecessary in modern browsers when sites
    /// implement a strong Content-Security-Policy that disables the use of
    /// inline JavaScript ('unsafe-inline'), they can still provide protections
    /// for users of older web browsers that don't yet support CSP.
    (XXssProtection, X_XSS_PROTECTION, "x-xss-protection", 0xc813b7f67f5e69ee);
}

/// Valid header name characters
///
///       field-name     = token
///       token          = 1*<any CHAR except CTLs or separators>
///       separators     = "(" | ")" | "<" | ">" | "@"
///                      | "," | ";" | ":" | "\" | <">
///                      | "/" | "[" | "]" | "?" | "="
///                      | "{" | "}" | SP | HT
const HEADER_CHARS: [u8; 256] = [
    //  0      1      2      3      4      5      6      7      8      9
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //   x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  1x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, //  2x
        0,     0,     0,  b'!',  b'"',  b'#',  b'$',  b'%',  b'&', b'\'', //  3x
        0,     0,  b'*',  b'+',     0,  b'-',  b'.',     0,  b'0',  b'1', //  4x
     b'2',  b'3',  b'4',  b'5',  b'6',  b'7',  b'8',  b'9',     0,     0, //  5x
        0,     0,     0,     0,     0,  b'a',  b'b',  b'c',  b'd',  b'e', //  6x
     b'f',  b'g',  b'h',  b'i',  b'j',  b'k',  b'l',  b'm',  b'n',  b'o', //  7x
     b'p',  b'q',  b'r',  b's',  b't',  b'u',  b'v',  b'w',  b'x',  b'y', //  8x
     b'z',     0,     0,     0,     0,  b'_',     0,  b'a',  b'b',  b'c', //  9x
     b'd',  b'e',  b'f',  b'g',  b'h',  b'i',  b'j',  b'k',  b'l',  b'm', // 10x
     b'n',  b'o',  b'p',  b'q',  b'r',  b's',  b't',  b'u',  b'v',  b'w', // 11x
     b'x',  b'y',  b'z',     0,  b'|',     0,  b'~',     0,     0,     0, // 12x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 13x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 14x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 15x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 16x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 17x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 18x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 19x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 20x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 21x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 22x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 23x
        0,     0,     0,     0,     0,     0,     0,     0,     0,     0, // 24x
        0,     0,     0,     0,     0,     0                              // 25x
];

macro_rules! eq {
    ($v:ident[$n:expr] == $a:tt) => {
        $v[$n] == $a
    };
    ($v:ident[$n:expr] == $a:tt $($rest:tt)+) => {
        $v[$n] == $a && eq!($v[($n+1)] == $($rest)+)
    };
    ($v:ident == $a:tt $($rest:tt)*) => {
        $v[0] == $a && eq!($v[1] == $($rest)*)
    };
}

macro_rules! to_lower {
    ($d:ident, $src:ident, 1) => { $d[0] = HEADER_CHARS[$src[0] as usize]; };
    ($d:ident, $src:ident, 2) => { to_lower!($d, $src, 1); $d[1] = HEADER_CHARS[$src[1] as usize]; };
    ($d:ident, $src:ident, 3) => { to_lower!($d, $src, 2); $d[2] = HEADER_CHARS[$src[2] as usize]; };
    ($d:ident, $src:ident, 4) => { to_lower!($d, $src, 3); $d[3] = HEADER_CHARS[$src[3] as usize]; };
    ($d:ident, $src:ident, 5) => { to_lower!($d, $src, 4); $d[4] = HEADER_CHARS[$src[4] as usize]; };
    ($d:ident, $src:ident, 6) => { to_lower!($d, $src, 5); $d[5] = HEADER_CHARS[$src[5] as usize]; };
    ($d:ident, $src:ident, 7) => { to_lower!($d, $src, 6); $d[6] = HEADER_CHARS[$src[6] as usize]; };
    ($d:ident, $src:ident, 8) => { to_lower!($d, $src, 7); $d[7] = HEADER_CHARS[$src[7] as usize]; };
    ($d:ident, $src:ident, 9) => { to_lower!($d, $src, 8); $d[8] = HEADER_CHARS[$src[8] as usize]; };
    ($d:ident, $src:ident, 10) => { to_lower!($d, $src, 9); $d[9] = HEADER_CHARS[$src[9] as usize]; };
    ($d:ident, $src:ident, 11) => { to_lower!($d, $src, 10); $d[10] = HEADER_CHARS[$src[10] as usize]; };
    ($d:ident, $src:ident, 12) => { to_lower!($d, $src, 11); $d[11] = HEADER_CHARS[$src[11] as usize]; };
    ($d:ident, $src:ident, 13) => { to_lower!($d, $src, 12); $d[12] = HEADER_CHARS[$src[12] as usize]; };
    ($d:ident, $src:ident, 14) => { to_lower!($d, $src, 13); $d[13] = HEADER_CHARS[$src[13] as usize]; };
    ($d:ident, $src:ident, 15) => { to_lower!($d, $src, 14); $d[14] = HEADER_CHARS[$src[14] as usize]; };
    ($d:ident, $src:ident, 16) => { to_lower!($d, $src, 15); $d[15] = HEADER_CHARS[$src[15] as usize]; };
    ($d:ident, $src:ident, 17) => { to_lower!($d, $src, 16); $d[16] = HEADER_CHARS[$src[16] as usize]; };
    ($d:ident, $src:ident, 18) => { to_lower!($d, $src, 17); $d[17] = HEADER_CHARS[$src[17] as usize]; };
    ($d:ident, $src:ident, 19) => { to_lower!($d, $src, 18); $d[18] = HEADER_CHARS[$src[18] as usize]; };
    ($d:ident, $src:ident, 20) => { to_lower!($d, $src, 19); $d[19] = HEADER_CHARS[$src[19] as usize]; };
    ($d:ident, $src:ident, 21) => { to_lower!($d, $src, 20); $d[20] = HEADER_CHARS[$src[20] as usize]; };
    ($d:ident, $src:ident, 22) => { to_lower!($d, $src, 21); $d[21] = HEADER_CHARS[$src[21] as usize]; };
    ($d:ident, $src:ident, 23) => { to_lower!($d, $src, 22); $d[22] = HEADER_CHARS[$src[22] as usize]; };
    ($d:ident, $src:ident, 24) => { to_lower!($d, $src, 23); $d[23] = HEADER_CHARS[$src[23] as usize]; };
    ($d:ident, $src:ident, 25) => { to_lower!($d, $src, 24); $d[24] = HEADER_CHARS[$src[24] as usize]; };
    ($d:ident, $src:ident, 26) => { to_lower!($d, $src, 25); $d[25] = HEADER_CHARS[$src[25] as usize]; };
    ($d:ident, $src:ident, 27) => { to_lower!($d, $src, 26); $d[26] = HEADER_CHARS[$src[26] as usize]; };
    ($d:ident, $src:ident, 28) => { to_lower!($d, $src, 27); $d[27] = HEADER_CHARS[$src[27] as usize]; };
    ($d:ident, $src:ident, 29) => { to_lower!($d, $src, 28); $d[28] = HEADER_CHARS[$src[28] as usize]; };
    ($d:ident, $src:ident, 30) => { to_lower!($d, $src, 29); $d[29] = HEADER_CHARS[$src[29] as usize]; };
    ($d:ident, $src:ident, 31) => { to_lower!($d, $src, 30); $d[30] = HEADER_CHARS[$src[30] as usize]; };
    ($d:ident, $src:ident, 32) => { to_lower!($d, $src, 31); $d[31] = HEADER_CHARS[$src[31] as usize]; };
    ($d:ident, $src:ident, 33) => { to_lower!($d, $src, 32); $d[32] = HEADER_CHARS[$src[32] as usize]; };
    ($d:ident, $src:ident, 34) => { to_lower!($d, $src, 33); $d[33] = HEADER_CHARS[$src[33] as usize]; };
    ($d:ident, $src:ident, 35) => { to_lower!($d, $src, 34); $d[34] = HEADER_CHARS[$src[34] as usize]; };
}

macro_rules! validate_chars {
    ($buf:ident) => {{
        if $buf.iter().any(|&b| b == 0) {
            return Err(FromBytesError::new());
        }
    }};
}

macro_rules! parse_hdr {
    ($data:ident, $res:ident, $standard:expr, $short:expr, $long: expr) => {{
        use self::StandardHeader::*;

        let len = $data.len();

        match len {
            0 => {
                return Err(FromBytesError::new());
            }
            2 => {
                let mut b: [u8; 2] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 2);

                if eq!(b == b't' b'e') {
                    let $res = Te;
                    return $standard;
                } else if eq!(b == b't' b'k') {
                    let $res = Tk;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            3 => {
                let mut b: [u8; 3] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 3);

                if eq!(b == b'a' b'g' b'e') {
                    let $res = Age;
                    return $standard;
                } else if eq!(b == b't' b's' b'v') {
                    let $res = Tsv;
                    return $standard;
                } else if eq!(b == b'v' b'i' b'a') {
                    let $res = Via;
                    return $standard;
                } else if eq!(b == b'd' b'n' b't') {
                    let $res = Dnt;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            4 => {
                let mut b: [u8; 4] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 4);

                if eq!(b == b'd' b'a' b't' b'e') {
                    let $res = Date;
                    return $standard;
                } else if eq!(b == b'e' b't' b'a' b'g') {
                    let $res = Etag;
                    return $standard;
                } else if eq!(b == b'f' b'r' b'o' b'm') {
                    let $res = From;
                    return $standard;
                } else if eq!(b == b'h' b'o' b's' b't') {
                    let $res = Host;
                    return $standard;
                } else if eq!(b == b'l' b'i' b'n' b'k') {
                    let $res = Link;
                    return $standard;
                } else if eq!(b == b'v' b'a' b'r' b'y') {
                    let $res = Vary;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            5 => {
                let mut b: [u8; 5] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 5);

                if eq!(b == b'a' b'l' b'l' b'o' b'w') {
                    let $res = Allow;
                    return $standard;
                } else if eq!(b == b'r' b'a' b'n' b'g' b'e') {
                    let $res = Range;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            6 => {
                let mut b: [u8; 6] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 6);

                if eq!(b == b'a' b'c' b'c' b'e' b'p' b't') {
                    let $res = Accept;
                    return $standard;
                } else if eq!(b == b'c' b'o' b'o' b'k' b'i' b'e') {
                    let $res = Cookie;
                    return $standard;
                } else if eq!(b == b'e' b'x' b'p' b'e' b'c' b't') {
                    let $res = Expect;
                    return $standard;
                } else if eq!(b == b'o' b'r' b'i' b'g' b'i' b'n') {
                    let $res = Origin;
                    return $standard;
                } else if eq!(b == b'p' b'r' b'a' b'g' b'm' b'a') {
                    let $res = Pragma;
                    return $standard;
                } if b[0] == b's' {
                    if eq!(b[1] == b'e' b'r' b'v' b'e' b'r') {
                        let $res = Server;
                        return $standard;
                    }
                }

                {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            7 => {
                let mut b: [u8; 7] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 7);

                if eq!(b == b'a' b'l' b't' b'-' b's' b'v' b'c') {
                    let $res = AltSvc;
                    return $standard;
                } else if eq!(b == b'e' b'x' b'p' b'i' b'r' b'e' b's') {
                    let $res = Expires;
                    return $standard;
                } else if eq!(b == b'r' b'e' b'f' b'e' b'r' b'e' b'r') {
                    let $res = Referer;
                    return $standard;
                } else if eq!(b == b'r' b'e' b'f' b'r' b'e' b's' b'h') {
                    let $res = Refresh;
                    return $standard;
                } else if eq!(b == b't' b'r' b'a' b'i' b'l' b'e' b'r') {
                    let $res = Trailer;
                    return $standard;
                } else if eq!(b == b'u' b'p' b'g' b'r' b'a' b'd' b'e') {
                    let $res = Upgrade;
                    return $standard;
                } else if eq!(b == b'w' b'a' b'r' b'n' b'i' b'n' b'g') {
                    let $res = Warning;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            8 => {
                let mut b: [u8; 8] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 8);

                if eq!(b == b'i' b'f' b'-') {
                    if eq!(b[3] == b'm' b'a' b't' b'c' b'h') {
                        let $res = IfMatch;
                        return $standard;
                    } else if eq!(b[3] == b'r' b'a' b'n' b'g' b'e') {
                        let $res = IfRange;
                        return $standard;
                    }
                } else if eq!(b == b'l' b'o' b'c' b'a' b't' b'i' b'o' b'n') {
                    let $res = Location;
                    return $standard;
                }

                {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            9 => {
                let mut b: [u8; 9] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 9);

                if eq!(b == b'f' b'o' b'r' b'w' b'a' b'r' b'd' b'e' b'd') {
                    let $res = Forwarded;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            10 => {
                let mut b: [u8; 10] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 10);

                if eq!(b == b'c' b'o' b'n' b'n' b'e' b'c' b't' b'i' b'o' b'n') {
                    let $res = Connection;
                    return $standard;
                } else if eq!(b == b's' b'e' b't' b'-' b'c' b'o' b'o' b'k' b'i' b'e') {
                    let $res = SetCookie;
                    return $standard;
                } else if eq!(b == b'u' b's' b'e' b'r' b'-' b'a' b'g' b'e' b'n' b't') {
                    let $res = UserAgent;
                    return $standard;
                } else if eq!(b == b'k' b'e' b'e' b'p' b'-' b'a' b'l' b'i' b'v' b'e') {
                    let $res = KeepAlive;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            11 => {
                let mut b: [u8; 11] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 11);

                if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-' b'm' b'd' b'5') {
                    let $res = ContentMd5;
                    return $standard;
                } else if eq!(b == b'r' b'e' b't' b'r' b'y' b'-' b'a' b'f' b't' b'e' b'r') {
                    let $res = RetryAfter;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            12 => {
                let mut b: [u8; 12] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 12);

                if eq!(b == b'a' b'c' b'c' b'e' b'p' b't' b'-' b'p' b'a' b't' b'c' b'h') {
                    let $res = AcceptPatch;
                    return $standard;
                } else if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-' b't' b'y' b'p' b'e') {
                    let $res = ContentType;
                    return $standard;
                } else if eq!(b == b'm' b'a' b'x' b'-' b'f' b'o' b'r' b'w' b'a' b'r' b'd' b's') {
                    let $res = MaxForwards;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            13 => {
                let mut b: [u8; 13] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 13);

                if b[0] == b'a' {
                    if eq!(b[1] == b'c' b'c' b'e' b'p' b't' b'-' b'r' b'a' b'n' b'g' b'e' b's') {
                        let $res = AcceptRanges;
                        return $standard;
                    } else if eq!(b[1] == b'u' b't' b'h' b'o' b'r' b'i' b'z' b'a' b't' b'i' b'o' b'n') {
                        let $res = Authorization;
                        return $standard;
                    }
                } else if b[0] == b'c' {
                    if eq!(b[1] == b'a' b'c' b'h' b'e' b'-' b'c' b'o' b'n' b't' b'r' b'o' b'l') {
                        let $res = CacheControl;
                        return $standard;
                    } else if eq!(b[1] == b'o' b'n' b't' b'e' b'n' b't' b'-' b'r' b'a' b'n' b'g' b'e' ) {
                        let $res = ContentRange;
                        return $standard;
                    }
                } else if eq!(b == b'i' b'f' b'-' b'n' b'o' b'n' b'e' b'-' b'm' b'a' b't' b'c' b'h') {
                    let $res = IfNoneMatch;
                    return $standard;
                } else if eq!(b == b'l' b'a' b's' b't' b'-' b'm' b'o' b'd' b'i' b'f' b'i' b'e' b'd') {
                    let $res = LastModified;
                    return $standard;
                }

                {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            14 => {
                let mut b: [u8; 14] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 14);

                if eq!(b == b'a' b'c' b'c' b'e' b'p' b't' b'-' b'c' b'h' b'a' b'r' b's' b'e' b't') {
                    let $res = AcceptCharset;
                    return $standard;
                } else if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-' b'l' b'e' b'n' b'g' b't' b'h') {
                    let $res = ContentLength;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            15 => {
                let mut b: [u8; 15] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 15);

                if eq!(b == b'a' b'c' b'c' b'e' b'p' b't' b'-') { // accept-
                    if eq!(b[7] == b'e' b'n' b'c' b'o' b'd' b'i' b'n' b'g') {
                        let $res = AcceptEncoding;
                        return $standard;
                    } else if eq!(b[7] == b'l' b'a' b'n' b'g' b'u' b'a' b'g' b'e') {
                        let $res = AcceptLanguage;
                        return $standard;
                    }
                } else if eq!(b == b'p' b'u' b'b' b'l' b'i' b'c' b'-' b'k' b'e' b'y' b'-' b'p' b'i' b'n' b's') {
                    let $res = PublicKeyPins;
                    return $standard;
                } else if eq!(b == b'x' b'-' b'f' b'r' b'a' b'm' b'e' b'-' b'o' b'p' b't' b'i' b'o' b'n' b's') {
                    let $res = XFrameOptions;
                    return $standard;
                }
                else if eq!(b == b'r' b'e' b'f' b'e' b'r' b'r' b'e' b'r' b'-' b'p' b'o' b'l' b'i' b'c' b'y') {
                    let $res = ReferrerPolicy;
                    return $standard;
                }

                {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            16 => {
                let mut b: [u8; 16] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 16);

                if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-') {
                    if eq!(b[8] == b'l' b'a' b'n' b'g' b'u' b'a' b'g' b'e') {
                        let $res = ContentLanguage;
                        return $standard;
                    } else if eq!(b[8] == b'l' b'o' b'c' b'a' b't' b'i' b'o' b'n') {
                        let $res = ContentLocation;
                        return $standard;
                    } else if eq!(b[8] == b'e' b'n' b'c' b'o' b'd' b'i' b'n' b'g') {
                        let $res = ContentEncoding;
                        return $standard;
                    }
                } else if eq!(b == b'w' b'w' b'w' b'-' b'a' b'u' b't' b'h' b'e' b'n' b't' b'i' b'c' b'a' b't' b'e') {
                    let $res = WwwAuthenticate;
                    return $standard;
                } else if eq!(b == b'x' b'-' b'x' b's' b's' b'-' b'p' b'r' b'o' b't' b'e' b'c' b't' b'i' b'o' b'n') {
                    let $res = XXssProtection;
                    return $standard;
                }

                let $res = &b[..];
                validate_chars!($res);
                return $short;
            }
            17 => {
                let mut b: [u8; 17] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 17);

                if eq!(b == b't' b'r' b'a' b'n' b's' b'f' b'e' b'r' b'-' b'e' b'n' b'c' b'o' b'd' b'i' b'n' b'g') {
                    let $res = TransferEncoding;
                    return $standard;
                } else if eq!(b == b'i' b'f' b'-' b'm' b'o' b'd' b'i' b'f' b'i' b'e' b'd' b'-' b's' b'i' b'n' b'c' b'e') {
                    let $res = IfModifiedSince;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            18 => {
                let mut b: [u8; 18] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 18);

                if eq!(b == b'p' b'r' b'o' b'x' b'y' b'-' b'a' b'u' b't' b'h' b'e' b'n' b't' b'i' b'c' b'a' b't' b'e') {
                    let $res = ProxyAuthenticate;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            19 => {
                let mut b: [u8; 19] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 19);

                if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-' b'd' b'i' b's' b'p' b'o' b's' b'i' b't' b'i' b'o' b'n') {
                    let $res = ContentDisposition;
                    return $standard;
                } else if eq!(b == b'i' b'f' b'-' b'u' b'n' b'm' b'o' b'd' b'i' b'f' b'i' b'e' b'd' b'-' b's' b'i' b'n' b'c' b'e') {
                    let $res = IfUnmodifiedSince;
                    return $standard;
                } else if eq!(b == b'p' b'r' b'o' b'x' b'y' b'-' b'a' b'u' b't' b'h' b'o' b'r' b'i' b'z' b'a' b't' b'i' b'o' b'n') {
                    let $res = ProxyAuthorization;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            22 => {
                let mut b: [u8; 22] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 22);

                if eq!(b == b'a' b'c' b'c' b'e' b's' b's' b'-' b'c' b'o' b'n' b't' b'r' b'o' b'l' b'-' b'm' b'a' b'x' b'-' b'a' b'g' b'e') {
                    let $res = AccessControlMaxAge;
                    return $standard;
                } else if eq!(b == b'x' b'-' b'c' b'o' b'n' b't' b'e' b'n' b't' b'-' b't' b'y' b'p' b'e' b'-' b'o' b'p' b't' b'i' b'o' b'n' b's') {
                    let $res = XContentTypeOptions;
                    return $standard;
                } else if eq!(b == b'x' b'-' b'd' b'n' b's' b'-' b'p' b'r' b'e' b'f' b'e' b't' b'c' b'h' b'-' b'c' b'o' b'n' b't' b'r' b'o' b'l') {
                    let $res = XDnsPrefetchControl;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            23 => {
                let mut b: [u8; 23] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 23);

                if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-' b's' b'e' b'c' b'u' b'r' b'i' b't' b'y' b'-' b'p' b'o' b'l' b'i' b'c' b'y') {
                    let $res = ContentSecurityPolicy;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            25 => {
                let mut b: [u8; 25] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 25);

                if eq!(b == b's' b't' b'r' b'i' b'c' b't' b'-' b't' b'r' b'a' b'n' b's' b'p' b'o' b'r' b't' b'-' b's' b'e' b'c' b'u' b'r' b'i' b't' b'y') {
                    let $res = StrictTransportSecurity;
                    return $standard;
                } else if eq!(b == b'u' b'p' b'g' b'r' b'a' b'd' b'e' b'-' b'i' b'n' b's' b'e' b'c' b'u' b'r' b'e' b'-' b'r' b'e' b'q' b'u' b'e' b's' b't' b's') {
                    let $res = UpgradeInsecureRequests;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            27 => {
                let mut b: [u8; 27] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 27);

                if eq!(b == b'a' b'c' b'c' b'e' b's' b's' b'-' b'c' b'o' b'n' b't' b'r' b'o' b'l' b'-' b'a' b'l' b'l' b'o' b'w' b'-' b'o' b'r' b'i' b'g' b'i' b'n') {
                    let $res = AccessControlAllowOrigin;
                    return $standard;
                } else if eq!(b == b'p' b'u' b'b' b'l' b'i' b'c' b'-' b'k' b'e' b'y' b'-' b'p' b'i' b'n' b's' b'-' b'r' b'e' b'p' b'o' b'r' b't' b'-' b'o' b'n' b'l' b'y') {
                    let $res = PublicKeyPinsReportOnly;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            28 => {
                let mut b: [u8; 28] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 28);

                if eq!(b == b'a' b'c' b'c' b'e' b's' b's' b'-' b'c' b'o' b'n' b't' b'r' b'o' b'l' b'-' b'a' b'l' b'l' b'o' b'w' b'-') {
                    if eq!(b[21] == b'h' b'e' b'a' b'd' b'e' b'r' b's') {
                        let $res = AccessControlAllowHeaders;
                        return $standard;
                    } else if eq!(b[21] == b'm' b'e' b't' b'h' b'o' b'd' b's') {
                        let $res = AccessControlAllowMethods;
                        return $standard;
                    }
                }

                let $res = &b[..];
                validate_chars!($res);
                return $short;
            }
            29 => {
                let mut b: [u8; 29] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 29);

                if eq!(b == b'a' b'c' b'c' b'e' b's' b's' b'-' b'c' b'o' b'n' b't' b'r' b'o' b'l' b'-') {
                    if eq!(b[15] == b'e' b'x' b'p' b'o' b's' b'e' b'-' b'h' b'e' b'a' b'd' b'e' b'r' b's') {
                        let $res = AccessControlExposeHeaders;
                        return $standard;
                    } else if eq!(b[15] == b'r' b'e' b'q' b'u' b'e' b's' b't' b'-' b'm' b'e' b't' b'h' b'o' b'd') {
                        let $res = AccessControlRequestMethod;
                        return $standard;
                    }
                }

                let $res = &b[..];
                validate_chars!($res);
                return $short;
            }
            30 => {
                let mut b: [u8; 30] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 30);

                if eq!(b == b'a' b'c' b'c' b'e' b's' b's' b'-' b'c' b'o' b'n' b't' b'r' b'o' b'l' b'-' b'r' b'e' b'q' b'u' b'e' b's' b't' b'-' b'h' b'e' b'a' b'd' b'e' b'r' b's') {
                    let $res = AccessControlRequestHeaders;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            32 => {
                let mut b: [u8; 32] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 32);

                if eq!(b == b'a' b'c' b'c' b'e' b's' b's' b'-' b'c' b'o' b'n' b't' b'r' b'o' b'l' b'-' b'a' b'l' b'l' b'o' b'w' b'-' b'c' b'r' b'e' b'd' b'e' b'n' b't' b'i' b'a' b'l' b's') {
                    let $res = AccessControlAllowCredentials;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            35 => {
                let mut b: [u8; 35] = unsafe { mem::uninitialized() };

                to_lower!(b, $data, 35);

                if eq!(b == b'c' b'o' b'n' b't' b'e' b'n' b't' b'-' b's' b'e' b'c' b'u' b'r' b'i' b't' b'y' b'-' b'p' b'o' b'l' b'i' b'c' b'y' b'-' b'r' b'e' b'p' b'o' b'r' b't' b'-' b'o' b'n' b'l' b'y') {
                    let $res = ContentSecurityPolicyReportOnly;
                    return $standard;
                } else {
                    let $res = &b[..];
                    validate_chars!($res);
                    return $short;
                }
            }
            _ => {
                if 0 == len & !(64-1) {
                    let mut buf: [u8; 64] = unsafe { ::std::mem::uninitialized() };

                    for i in 0..len {
                        buf[i] = HEADER_CHARS[$data[i] as usize];
                    }

                    let $res = &buf[..len];
                    validate_chars!($res);
                    return $short;
                } else {
                    let $res = $data;
                    return $long;
                }
            }
        }
    }};
}

impl HeaderName {
    /// Converts a slice of bytes to an HTTP header name.
    ///
    /// This function normalizes the input.
    pub fn from_bytes(src: &[u8]) -> Result<HeaderName, FromBytesError> {
        parse_hdr!(
            src,
            res,
            Ok(res.into()),
            {
                let buf = Bytes::from(&res[..]);
                let val = unsafe { ByteStr::from_utf8_unchecked(buf) };
                Ok(Custom(val).into())
            },
            {
                use bytes::{BufMut};
                let mut dst = BytesMut::with_capacity(res.len());

                for b in res.iter() {
                    let b = HEADER_CHARS[*b as usize];

                    if b == 0 {
                        return Err(FromBytesError::new());
                    }

                    dst.put(b);
                }

                let val = unsafe { ByteStr::from_utf8_unchecked(dst.freeze()) };

                Ok(Custom(val).into())
            })
    }

    /// Returns a `str` representation of the header.
    ///
    /// The returned string will always be lower case.
    pub fn as_str(&self) -> &str {
        match self.inner {
            Repr::Standard(v) => v.as_str(),
            Repr::Custom(ref v) => &*v.0,
        }
    }
}

impl FromStr for HeaderName {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<HeaderName, FromStrError> {
        HeaderName::from_bytes(s.as_bytes())
            .map_err(|_| FromStrError {
                _priv: (),
            })
    }
}

impl FastHash for HeaderName {
    #[inline]
    fn fast_hash(&self) -> u64 {
        match self.inner {
            Repr::Standard(s) => s.fast_hash(),
            Repr::Custom(ref b) => fast_hash::fast_hash(b.0.as_bytes()),
        }
    }
}

impl<'a> FastHash for &'a HeaderName {
    #[inline]
    fn fast_hash(&self) -> u64 {
        (**self).fast_hash()
    }
}

impl AsRef<str> for HeaderName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for HeaderName {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl fmt::Debug for HeaderName {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), fmt)
    }
}

impl FromBytesError {
    fn new() -> FromBytesError {
        FromBytesError { _priv: () }
    }
}

impl<'a> From<&'a HeaderName> for HeaderName {
    fn from(src: &'a HeaderName) -> HeaderName {
        src.clone()
    }
}

impl From<StandardHeader> for HeaderName {
    fn from(src: StandardHeader) -> HeaderName {
        HeaderName {
            inner: Repr::Standard(src),
        }
    }
}

impl From<Custom> for HeaderName {
    fn from(src: Custom) -> HeaderName {
        HeaderName { inner: Repr::Custom(src) }
    }
}

impl<'a> PartialEq<&'a HeaderName> for HeaderName {
    #[inline]
    fn eq(&self, other: &&'a HeaderName) -> bool {
        *self == **other
    }
}

// ===== HdrName =====

impl<'a> HdrName<'a> {
    fn custom(buf: &'a [u8], lower: bool) -> HdrName<'a> {
        HdrName {
            inner: Repr::Custom(MaybeLower {
                buf: buf,
                lower: lower,
            }),
        }
    }

    pub fn from_bytes<F, U>(hdr: &[u8], f: F) -> Result<U, FromBytesError>
        where F: FnOnce(HdrName) -> U,
    {
        parse_hdr!(
            hdr,
            res,
            {
                Ok(f(HdrName {
                    inner: Repr::Standard(res),
                }))
            },
            {
                Ok(f(HdrName::custom(res, true)))
            },
            {
                Ok(f(HdrName::custom(res, false)))
            })
    }
}

impl<'a> FastHash for HdrName<'a> {
    #[inline]
    fn fast_hash(&self) -> u64 {
        match self.inner {
            Repr::Standard(s) => s.fast_hash(),
            Repr::Custom(ref maybe_lower) => {
                if maybe_lower.lower {
                    fast_hash::fast_hash(maybe_lower.buf)
                } else {
                    let mut buf = [0u8; 8];
                    let mut src = maybe_lower.buf;

                    let mut hasher = FastHasher::new();

                    while src.len() >= 8 {
                        to_lower!(buf, src, 8);

                        hasher.hash(&buf);

                        src = &src[8..];
                    }

                    for (i, &b) in src.iter().enumerate() {
                        buf[i] = HEADER_CHARS[b as usize];
                    }

                    hasher.finish(&buf[..src.len()])
                }
            }
        }
    }
}

impl<'a> From<HdrName<'a>> for HeaderName {
    fn from(src: HdrName<'a>) -> HeaderName {
        match src.inner {
            Repr::Standard(s) => {
                HeaderName {
                    inner: Repr::Standard(s),
                }
            }
            Repr::Custom(maybe_lower) => {
                if maybe_lower.lower {
                    let buf = Bytes::from(&maybe_lower.buf[..]);
                    let byte_str = unsafe { ByteStr::from_utf8_unchecked(buf) };

                    HeaderName {
                        inner: Repr::Custom(Custom(byte_str)),
                    }
                } else {
                    use bytes::{BufMut};
                    let mut dst = BytesMut::with_capacity(maybe_lower.buf.len());

                    for b in maybe_lower.buf.iter() {
                        dst.put(HEADER_CHARS[*b as usize]);
                    }

                    let buf = unsafe { ByteStr::from_utf8_unchecked(dst.freeze()) };

                    HeaderName {
                        inner: Repr::Custom(Custom(buf)),
                    }
                }
            }
        }
    }
}

impl<'a> PartialEq<HdrName<'a>> for HeaderName {
    #[inline]
    fn eq(&self, other: &HdrName<'a>) -> bool {
        match self.inner {
            Repr::Standard(a) => {
                match other.inner {
                    Repr::Standard(b) => a == b,
                    _ => false,
                }
            }
            Repr::Custom(Custom(ref a)) => {
                match other.inner {
                    Repr::Custom(ref b) => {
                        if b.lower {
                            a.as_bytes() == b.buf
                        } else {
                            eq_ignore_ascii_case(a.as_bytes(), b.buf)
                        }
                    }
                    _ => false,
                }
            }
        }
    }
}

// ===== Custom =====

impl Hash for Custom {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        for b in self.0.as_bytes() {
            b.hash(hasher);
        }
    }
}

// ===== MaybeLower =====

impl<'a> Hash for MaybeLower<'a> {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        if self.lower {
            for &b in self.buf {
                b.hash(hasher);
            }
        } else {
            for &b in self.buf {
                HEADER_CHARS[b as usize].hash(hasher);
            }
        }
    }
}

// Assumes that the left hand side is already lower case
#[inline]
fn eq_ignore_ascii_case(lower: &[u8], s: &[u8]) -> bool {
    if lower.len() != s.len() {
        return false;
    }

    lower.iter().zip(s).all(|(a, b)| {
        *a == HEADER_CHARS[*b as usize]
    })
}

#[test]
fn test_parse_standard_headers() {
    use self::StandardHeader::*;

    const HEADERS: &'static [(StandardHeader, &'static str)] = &[
        (Accept, "accept"),
        (AcceptCharset, "accept-charset"),
        (AcceptEncoding, "accept-encoding"),
        (AcceptLanguage, "accept-language"),
        (AcceptPatch, "accept-patch"),
        (AcceptRanges, "accept-ranges"),
        (AccessControlAllowCredentials, "access-control-allow-credentials"),
        (AccessControlAllowHeaders, "access-control-allow-headers"),
        (AccessControlAllowMethods, "access-control-allow-methods"),
        (AccessControlAllowOrigin, "access-control-allow-origin"),
        (AccessControlExposeHeaders, "access-control-expose-headers"),
        (AccessControlMaxAge, "access-control-max-age"),
        (AccessControlRequestHeaders, "access-control-request-headers"),
        (AccessControlRequestMethod, "access-control-request-method"),
        (Age, "age"),
        (Allow, "allow"),
        (AltSvc, "alt-svc"),
        (Authorization, "authorization"),
        (CacheControl, "cache-control"),
        (Connection, "connection"),
        (ContentDisposition, "content-disposition"),
        (ContentEncoding, "content-encoding"),
        (ContentLanguage, "content-language"),
        (ContentLength, "content-length"),
        (ContentLocation, "content-location"),
        (ContentMd5, "content-md5"),
        (ContentRange, "content-range"),
        (ContentSecurityPolicy, "content-security-policy"),
        (ContentSecurityPolicyReportOnly, "content-security-policy-report-only"),
        (ContentType, "content-type"),
        (Cookie, "cookie"),
        (Dnt, "dnt"),
        (Date, "date"),
        (Etag, "etag"),
        (Expect, "expect"),
        (Expires, "expires"),
        (Forwarded, "forwarded"),
        (From, "from"),
        (Host, "host"),
        (IfMatch, "if-match"),
        (IfModifiedSince, "if-modified-since"),
        (IfNoneMatch, "if-none-match"),
        (IfRange, "if-range"),
        (IfUnmodifiedSince, "if-unmodified-since"),
        (LastModified, "last-modified"),
        (KeepAlive, "keep-alive"),
        (Link, "link"),
        (Location, "location"),
        (MaxForwards, "max-forwards"),
        (Origin, "origin"),
        (Pragma, "pragma"),
        (ProxyAuthenticate, "proxy-authenticate"),
        (ProxyAuthorization, "proxy-authorization"),
        (PublicKeyPins, "public-key-pins"),
        (PublicKeyPinsReportOnly, "public-key-pins-report-only"),
        (Range, "range"),
        (Referer, "referer"),
        (ReferrerPolicy, "referrer-policy"),
        (Refresh, "refresh"),
        (RetryAfter, "retry-after"),
        (Server, "server"),
        (SetCookie, "set-cookie"),
        (StrictTransportSecurity, "strict-transport-security"),
        (Te, "te"),
        (Tk, "tk"),
        (Trailer, "trailer"),
        (TransferEncoding, "transfer-encoding"),
        (Tsv, "tsv"),
        (UserAgent, "user-agent"),
        (Upgrade, "upgrade"),
        (UpgradeInsecureRequests, "upgrade-insecure-requests"),
        (Vary, "vary"),
        (Via, "via"),
        (Warning, "warning"),
        (WwwAuthenticate, "www-authenticate"),
        (XContentTypeOptions, "x-content-type-options"),
        (XDnsPrefetchControl, "x-dns-prefetch-control"),
        (XFrameOptions, "x-frame-options"),
        (XXssProtection, "x-xss-protection"),
    ];

    for &(std, name) in HEADERS {
        // Test lower case
        assert_eq!(HeaderName::from_bytes(name.as_bytes()).unwrap(), HeaderName::from(std));

        // Test upper case
        let upper = name.to_uppercase().to_string();
        assert_eq!(HeaderName::from_bytes(upper.as_bytes()).unwrap(), HeaderName::from(std));
    }
}

#[test]
fn test_parse_invalid_headers() {
    for i in 0..128 {
        let hdr = vec![1u8; i];
        assert!(HeaderName::from_bytes(&hdr).is_err(), "{} invalid header chars did not fail", i);
    }
}

#[test]
fn test_from_hdr_name() {
    use self::StandardHeader::Vary;

    let name = HeaderName::from(HdrName {
        inner: Repr::Standard(Vary),
    });

    assert_eq!(name.inner, Repr::Standard(Vary));

    let name = HeaderName::from(HdrName {
        inner: Repr::Custom(MaybeLower {
            buf: b"hello-world",
            lower: true,
        }),
    });

    assert_eq!(name.inner, Repr::Custom(Custom(ByteStr::from_static("hello-world"))));

    let name = HeaderName::from(HdrName {
        inner: Repr::Custom(MaybeLower {
            buf: b"Hello-World",
            lower: false,
        }),
    });

    assert_eq!(name.inner, Repr::Custom(Custom(ByteStr::from_static("hello-world"))));
}

#[test]
fn test_eq_hdr_name() {
    use self::StandardHeader::Vary;

    let a = HeaderName { inner: Repr::Standard(Vary) };
    let b = HdrName { inner: Repr::Standard(Vary) };

    assert_eq!(a, b);

    let a = HeaderName { inner: Repr::Custom(Custom(ByteStr::from_static("vaary"))) };
    assert_ne!(a, b);

    let b = HdrName { inner: Repr::Custom(MaybeLower {
        buf: b"vaary",
        lower: true,
    })};

    assert_eq!(a, b);

    let b = HdrName { inner: Repr::Custom(MaybeLower {
        buf: b"vaary",
        lower: false,
    })};

    assert_eq!(a, b);

    let b = HdrName { inner: Repr::Custom(MaybeLower {
        buf: b"VAARY",
        lower: false,
    })};

    assert_eq!(a, b);

    let a = HeaderName { inner: Repr::Standard(Vary) };
    assert_ne!(a, b);
}

#[test]
fn test_hashing() {
    use self::StandardHeader::*;
    use std::collections::hash_map::DefaultHasher;

    fn hash<T: Hash>(v: &T) -> u64 {
        let mut h = DefaultHasher::new();
        v.hash(&mut h);
        h.finish()
    }

    for &hdr in &[Accept, Age, Allow, Expect] {
        let a = HeaderName { inner: Repr::Standard(hdr) };
        let b = HdrName { inner: Repr::Standard(hdr) };

        assert_eq!(a.fast_hash(), b.fast_hash());
        assert_eq!(hash(&a), hash(&b));

        for &hdr2 in &[AcceptRanges, Connection, Etag] {
            let a2 = HeaderName { inner: Repr::Standard(hdr2) };
            let b2 = HdrName { inner: Repr::Standard(hdr2) };

            assert_ne!(a2.fast_hash(), b.fast_hash());
            assert_ne!(hash(&a2), hash(&b));

            assert_ne!(a.fast_hash(), b2.fast_hash());
            assert_ne!(hash(&a), hash(&b2));
        }
    }

    // Case insensitive hashing
    let a: HeaderName = "hello-world".parse().unwrap();
    let b = HdrName {
        inner: Repr::Custom(MaybeLower {
            buf: b"Hello-World",
            lower: false,
        }),
    };

    assert_eq!(a.fast_hash(), b.fast_hash());
    assert_eq!(hash(&a), hash(&b));

    let b = HdrName {
        inner: Repr::Custom(MaybeLower {
            buf: b"hello-waldo",
            lower: false,
        }),
    };

    assert_ne!(a.fast_hash(), b.fast_hash());
    assert_ne!(hash(&a), hash(&b));

    let b = HdrName {
        inner: Repr::Custom(MaybeLower {
            buf: b"hello-world",
            lower: false,
        }),
    };

    assert_eq!(a.fast_hash(), b.fast_hash());
    assert_eq!(hash(&a), hash(&b));

    let b = HdrName {
        inner: Repr::Custom(MaybeLower {
            buf: b"hello-world",
            lower: true,
        }),
    };

    assert_eq!(a.fast_hash(), b.fast_hash());
    assert_eq!(hash(&a), hash(&b));
}
