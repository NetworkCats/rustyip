use std::path::Path;

use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use rustyip::db;
use rustyip::handlers::AppState;
use rustyip::routes::build_router;

fn test_db_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = format!("{manifest_dir}/data/Merged-IP.mmdb");
    if !Path::new(&path).exists() {
        panic!("Test MMDB not found at {path}. Download it first.");
    }
    path
}

fn build_test_app() -> axum::Router {
    let reader = db::load_db(Path::new(&test_db_path())).expect("failed to load test DB");
    let shared_db = db::new_shared(reader);
    let state = AppState {
        db: shared_db,
        site_domain: "test.example.com".into(),
    };
    build_router(state)
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, String) {
    get_with_headers(app, uri, vec![]).await
}

async fn get_with_headers(
    app: &axum::Router,
    uri: &str,
    headers: Vec<(&str, &str)>,
) -> (StatusCode, String) {
    let mut builder = Request::builder().uri(uri).method("GET");
    for (k, v) in headers {
        builder = builder.header(k, v);
    }
    let request = builder.body(axum::body::Body::empty()).unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(body.to_vec()).unwrap();
    (status, text)
}

#[tokio::test]
async fn health_returns_200() {
    let app = build_test_app();
    let (status, _) = get(&app, "/health").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn json_endpoint_returns_full_info() {
    let app = build_test_app();
    let (status, body) = get(&app, "/json?ip=45.77.77.77").await;
    assert_eq!(status, StatusCode::OK);

    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"], "45.77.77.77");
    assert_eq!(json["asn"]["autonomous_system_number"], 20473);
    assert_eq!(
        json["asn"]["autonomous_system_organization"],
        "The Constant Company, LLC"
    );
    assert_eq!(json["country"]["iso_code"], "US");
    assert_eq!(json["city"]["names"]["en"], "Piscataway");
    assert_eq!(json["proxy"]["is_proxy"], true);
    assert_eq!(json["proxy"]["is_hosting"], true);
    assert_eq!(json["proxy"]["is_tor"], false);
}

#[tokio::test]
async fn json_endpoint_ipv6() {
    let app = build_test_app();
    let (status, body) = get(&app, "/json?ip=2606:4700:4700::1111").await;
    assert_eq!(status, StatusCode::OK);

    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"], "2606:4700:4700::1111");
    assert_eq!(json["asn"]["autonomous_system_number"], 13335);
}

#[tokio::test]
async fn ip_endpoint_returns_plain_text() {
    let app = build_test_app();
    let (status, body) = get(&app, "/ip?ip=8.8.8.8").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "8.8.8.8");
}

#[tokio::test]
async fn asn_endpoint() {
    let app = build_test_app();
    let (status, body) = get(&app, "/asn?ip=1.1.1.1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "AS13335");
}

#[tokio::test]
async fn org_endpoint() {
    let app = build_test_app();
    let (status, body) = get(&app, "/org?ip=1.1.1.1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "Cloudflare, Inc.");
}

#[tokio::test]
async fn country_endpoint() {
    let app = build_test_app();
    let (status, body) = get(&app, "/country?ip=45.77.77.77").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "United States");
}

#[tokio::test]
async fn city_endpoint() {
    let app = build_test_app();
    let (status, body) = get(&app, "/city?ip=45.77.77.77").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "Piscataway");
}

#[tokio::test]
async fn proxy_endpoint() {
    let app = build_test_app();
    let (status, body) = get(&app, "/proxy?ip=45.77.77.77").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "true");
}

#[tokio::test]
async fn vpn_endpoint() {
    let app = build_test_app();
    let (status, body) = get(&app, "/vpn?ip=45.77.77.77").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "false");
}

#[tokio::test]
async fn hosting_endpoint() {
    let app = build_test_app();
    let (status, body) = get(&app, "/hosting?ip=45.77.77.77").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "true");
}

#[tokio::test]
async fn tor_endpoint() {
    let app = build_test_app();
    let (status, body) = get(&app, "/tor?ip=45.77.77.77").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "false");
}

#[tokio::test]
async fn invalid_ip_returns_400() {
    let app = build_test_app();
    let (status, _) = get(&app, "/json?ip=not-an-ip").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cli_user_agent_returns_plain_ip() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/",
        vec![
            ("User-Agent", "curl/8.7.1"),
            ("CF-Connecting-IP", "1.2.3.4"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "1.2.3.4");
}

#[tokio::test]
async fn accept_json_returns_json() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/",
        vec![
            ("Accept", "application/json"),
            ("CF-Connecting-IP", "45.77.77.77"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"], "45.77.77.77");
}

#[tokio::test]
async fn browser_returns_html() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/?ip=45.77.77.77",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("45.77.77.77"));
    assert!(body.contains("Piscataway"));
    assert!(body.contains("AS20473"));
    assert!(body.contains("icon-check bool-true"));
    assert!(body.contains("icon-minus bool-false"));
}

#[tokio::test]
async fn root_without_cf_header_returns_error() {
    let app = build_test_app();
    let (status, _) = get_with_headers(
        &app,
        "/",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn json_current_ip_via_cf_header() {
    let app = build_test_app();
    let (status, body) =
        get_with_headers(&app, "/json", vec![("CF-Connecting-IP", "8.8.8.8")]).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"], "8.8.8.8");
    assert_eq!(json["asn"]["autonomous_system_number"], 15169);
}

#[tokio::test]
async fn wget_user_agent_returns_plain_ip() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/",
        vec![
            ("User-Agent", "Wget/1.21.4"),
            ("CF-Connecting-IP", "10.0.0.1"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "10.0.0.1");
}

#[tokio::test]
async fn ipv6_via_cf_header() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/",
        vec![
            ("User-Agent", "curl/8.0"),
            ("CF-Connecting-IP", "2606:4700:4700::1111"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "2606:4700:4700::1111");
}

// --- Edge case tests ---

#[tokio::test]
async fn empty_ip_query_falls_back_to_cf_header() {
    let app = build_test_app();
    let (status, body) =
        get_with_headers(&app, "/json?ip=", vec![("CF-Connecting-IP", "8.8.8.8")]).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"], "8.8.8.8");
}

#[tokio::test]
async fn whitespace_ip_query_falls_back_to_cf_header() {
    let app = build_test_app();
    let (status, body) =
        get_with_headers(&app, "/json?ip=%20", vec![("CF-Connecting-IP", "8.8.8.8")]).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"], "8.8.8.8");
}

#[tokio::test]
async fn ip_not_in_database_returns_500() {
    let app = build_test_app();
    // 0.0.0.0 is unlikely to be in a geo database
    let (status, _) = get(&app, "/json?ip=0.0.0.0").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn cli_with_query_param_returns_html() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/?ip=45.77.77.77",
        vec![
            ("User-Agent", "curl/8.7.1"),
            ("CF-Connecting-IP", "1.2.3.4"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // When CLI sends ?ip=, the shortcut is bypassed and HTML is returned
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("45.77.77.77"));
}

#[tokio::test]
async fn accept_json_with_query_param_returns_html() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/?ip=45.77.77.77",
        vec![
            ("Accept", "application/json"),
            ("CF-Connecting-IP", "8.8.8.8"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // When Accept: application/json is sent with ?ip=, the JSON shortcut is bypassed
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("45.77.77.77"));
}

#[tokio::test]
async fn ip_endpoint_via_cf_header() {
    let app = build_test_app();
    let (status, body) = get_with_headers(&app, "/ip", vec![("CF-Connecting-IP", "1.1.1.1")]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "1.1.1.1");
}

#[tokio::test]
async fn asn_endpoint_via_cf_header() {
    let app = build_test_app();
    let (status, body) =
        get_with_headers(&app, "/asn", vec![("CF-Connecting-IP", "1.1.1.1")]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "AS13335");
}

#[tokio::test]
async fn org_endpoint_via_cf_header() {
    let app = build_test_app();
    let (status, body) =
        get_with_headers(&app, "/org", vec![("CF-Connecting-IP", "1.1.1.1")]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "Cloudflare, Inc.");
}

#[tokio::test]
async fn country_endpoint_via_cf_header() {
    let app = build_test_app();
    let (status, body) =
        get_with_headers(&app, "/country", vec![("CF-Connecting-IP", "45.77.77.77")]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "United States");
}

#[tokio::test]
async fn invalid_ip_on_field_endpoints_returns_400() {
    let app = build_test_app();

    let endpoints = [
        "/ip", "/asn", "/org", "/country", "/city", "/proxy", "/vpn", "/hosting", "/tor",
    ];
    for endpoint in endpoints {
        let uri = format!("{endpoint}?ip=invalid");
        let (status, _) = get(&app, &uri).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "expected 400 for {endpoint}?ip=invalid"
        );
    }
}

#[tokio::test]
async fn field_endpoints_without_ip_or_header_return_400() {
    let app = build_test_app();

    let endpoints = [
        "/ip", "/asn", "/org", "/country", "/city", "/proxy", "/vpn", "/hosting", "/tor", "/json",
    ];
    for endpoint in endpoints {
        let (status, _) = get(&app, endpoint).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "expected 400 for {endpoint} without IP"
        );
    }
}
