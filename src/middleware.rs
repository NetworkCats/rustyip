//! Response mappers that apply security headers. They are installed with
//! `tower::util::MapResponseLayer`, which wraps the inner future without boxing it.

use axum::http::header::{self, HeaderValue};
use axum::response::Response;

pub fn security_headers(mut response: Response) -> Response {
    apply_common_security_headers(response.headers_mut());
    response.headers_mut().insert(
        header::HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    response
}

pub fn ipv4_domain_security_headers(mut response: Response) -> Response {
    apply_common_security_headers(response.headers_mut());
    response.headers_mut().insert(
        header::HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("cross-origin"),
    );
    response
}

fn apply_common_security_headers(headers: &mut axum::http::HeaderMap) {
    // Room for the common headers plus the per-domain ones, so the map grows
    // at most once instead of rehashing repeatedly while headers are added.
    headers.reserve(8);
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
    );
    headers.insert(
        header::HeaderName::from_static("x-permitted-cross-domain-policies"),
        HeaderValue::from_static("none"),
    );
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), interest-cohort=()"),
    );
}
