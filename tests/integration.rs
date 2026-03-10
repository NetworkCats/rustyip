use std::path::Path;

use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use rustyip::db;
use rustyip::handlers::{AppState, build_openapi_json};
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
    build_test_app_with_dev_mode(false)
}

fn build_test_app_with_dev_mode(dev_mode: bool) -> axum::Router {
    let reader = db::load_db(Path::new(&test_db_path())).expect("failed to load test DB");
    let shared_db = db::new_shared(reader);
    let site_domain: std::sync::Arc<str> = "test.example.com".into();
    let openapi_json = build_openapi_json(&site_domain);
    let ipv4_domain: std::sync::Arc<str> = "noipv6.test.example.com".into();
    let state = AppState {
        db: shared_db,
        site_domain,
        ipv4_domain,
        dev_mode,
        openapi_json,
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

async fn get_response(
    app: &axum::Router,
    uri: &str,
    headers: Vec<(&str, &str)>,
) -> axum::http::Response<axum::body::Body> {
    let mut builder = Request::builder().uri(uri).method("GET");
    for (k, v) in headers {
        builder = builder.header(k, v);
    }
    let request = builder.body(axum::body::Body::empty()).unwrap();
    app.clone().oneshot(request).await.unwrap()
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

// The root path for browser requests now redirects to /{lang}/
#[tokio::test]
async fn root_redirects_browser_to_lang_path() {
    let app = build_test_app();
    let response = get_response(
        &app,
        "/",
        vec![
            (
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            ),
            ("CF-Connecting-IP", "45.77.77.77"),
        ],
    )
    .await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        location.starts_with("/en"),
        "redirect should go to /en, got: {location}"
    );
}

#[tokio::test]
async fn root_redirects_to_japanese_for_ja_accept_language() {
    let app = build_test_app();
    let response = get_response(
        &app,
        "/",
        vec![
            ("User-Agent", "Mozilla/5.0"),
            ("Accept-Language", "ja,en;q=0.5"),
            ("CF-Connecting-IP", "1.1.1.1"),
        ],
    )
    .await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(location, "/ja");
}

#[tokio::test]
async fn browser_returns_html_at_lang_path() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=45.77.77.77",
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
async fn browser_returns_html_in_spanish() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/es?ip=45.77.77.77",
        vec![("User-Agent", "Mozilla/5.0")],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("lang=\"es\""));
    assert!(body.contains("45.77.77.77"));
}

#[tokio::test]
async fn browser_returns_html_in_arabic_with_rtl() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/ar?ip=45.77.77.77",
        vec![("User-Agent", "Mozilla/5.0")],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("lang=\"ar\""));
    assert!(body.contains("dir=\"rtl\""));
}

#[tokio::test]
async fn root_without_cf_header_redirects() {
    let app = build_test_app();
    let response = get_response(
        &app,
        "/",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    // Root should redirect to /en when no Accept-Language header
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn lang_path_without_cf_header_returns_error() {
    let app = build_test_app();
    let (status, _) = get_with_headers(
        &app,
        "/en",
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

#[tokio::test]
async fn python_requests_returns_plain_ip() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/",
        vec![
            ("User-Agent", "python-requests/2.31.0"),
            ("CF-Connecting-IP", "1.2.3.4"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "1.2.3.4");
}

#[tokio::test]
async fn go_http_client_returns_plain_ip() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/",
        vec![
            ("User-Agent", "Go-http-client/2.0"),
            ("CF-Connecting-IP", "1.2.3.4"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "1.2.3.4");
}

#[tokio::test]
async fn axios_returns_plain_ip() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/",
        vec![
            ("User-Agent", "axios/1.7.2"),
            ("CF-Connecting-IP", "1.2.3.4"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "1.2.3.4");
}

#[tokio::test]
async fn guzzle_returns_plain_ip() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/",
        vec![
            ("User-Agent", "GuzzleHttp/7.8.1 curl/8.4.0 PHP/8.3.3"),
            ("CF-Connecting-IP", "1.2.3.4"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "1.2.3.4");
}

#[tokio::test]
async fn reqwest_returns_plain_ip() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/",
        vec![
            ("User-Agent", "reqwest/0.12.4"),
            ("CF-Connecting-IP", "1.2.3.4"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "1.2.3.4");
}

#[tokio::test]
async fn http_lib_on_lang_path_returns_plain_ip() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en",
        vec![
            ("User-Agent", "python-requests/2.31.0"),
            ("CF-Connecting-IP", "1.2.3.4"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "1.2.3.4");
}

#[tokio::test]
async fn http_lib_with_query_param_returns_html() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=45.77.77.77",
        vec![
            ("User-Agent", "python-requests/2.31.0"),
            ("CF-Connecting-IP", "1.2.3.4"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("45.77.77.77"));
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
async fn non_public_ip_returns_400() {
    let app = build_test_app();
    let (status, body) = get(&app, "/json?ip=0.0.0.0").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, "Only public IP addresses can be queried");
}

#[tokio::test]
async fn cli_with_query_param_returns_html() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=45.77.77.77",
        vec![
            ("User-Agent", "curl/8.7.1"),
            ("CF-Connecting-IP", "1.2.3.4"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("45.77.77.77"));
}

#[tokio::test]
async fn accept_json_with_query_param_returns_html() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=45.77.77.77",
        vec![
            ("Accept", "application/json"),
            ("CF-Connecting-IP", "8.8.8.8"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
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

// --- SEO and accessibility tests ---

#[tokio::test]
async fn robots_txt_returns_valid_response() {
    let app = build_test_app();
    let (status, body) = get(&app, "/robots.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("User-agent: *"));
    assert!(body.contains("Disallow: /json"));
    assert!(body.contains("Disallow: /health"));
    assert!(body.contains("Sitemap: https://test.example.com/sitemap.xml"));
}

#[tokio::test]
async fn sitemap_xml_returns_valid_response() {
    let app = build_test_app();
    let (status, body) = get(&app, "/sitemap.xml").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<?xml version=\"1.0\""));
    assert!(body.contains("<loc>https://test.example.com/en</loc>"));
    assert!(body.contains("<loc>https://test.example.com/es</loc>"));
    assert!(body.contains("<loc>https://test.example.com/ja</loc>"));
    assert!(body.contains("hreflang=\"en\""));
    assert!(body.contains("hreflang=\"x-default\""));
    assert!(body.contains("hreflang=\"zh-Hant\""));
    assert!(body.contains("xmlns:xhtml"));
}

#[tokio::test]
async fn html_contains_seo_meta_tags() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en",
        vec![
            (
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            ),
            ("CF-Connecting-IP", "45.77.77.77"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<meta name=\"description\""));
    assert!(body.contains("<link rel=\"canonical\""));
    assert!(body.contains("<meta property=\"og:title\""));
    assert!(body.contains("<meta property=\"og:description\""));
    assert!(body.contains("<meta property=\"og:type\" content=\"website\""));
    assert!(body.contains("<meta name=\"twitter:card\" content=\"summary\""));
    assert!(body.contains("<meta name=\"theme-color\""));
    assert!(body.contains("hreflang=\"en\""));
    assert!(body.contains("hreflang=\"x-default\""));
    assert!(body.contains("hreflang=\"es\""));
    assert!(!body.contains("noindex"));
}

#[tokio::test]
async fn html_query_result_has_noindex() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=45.77.77.77",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<meta name=\"robots\" content=\"noindex, nofollow\">"));
    assert!(!body.contains("<meta name=\"description\""));
    assert!(!body.contains("<link rel=\"canonical\""));
    assert!(!body.contains("<meta property=\"og:title\""));
    assert!(!body.contains("hreflang="));
    assert!(body.contains("<meta name=\"theme-color\""));
}

#[tokio::test]
async fn html_uses_th_for_row_headers() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=45.77.77.77",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<th scope=\"row\">ASN</th>"));
    assert!(body.contains("<th scope=\"row\">Country</th>"));
    assert!(body.contains("autocapitalize=\"none\""));
}

#[tokio::test]
async fn html_contains_lang_attribute() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/de?ip=45.77.77.77",
        vec![("User-Agent", "Mozilla/5.0")],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("lang=\"de\""));
}

#[tokio::test]
async fn html_contains_language_selector() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=45.77.77.77",
        vec![("User-Agent", "Mozilla/5.0")],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("lang-select"));
    assert!(body.contains("value=\"/en\""));
    assert!(body.contains("value=\"/es\""));
    assert!(body.contains("value=\"/ja\""));
    assert!(body.contains("value=\"/zh-Hant\""));
    assert!(body.contains("value=\"/ar\""));
}

#[tokio::test]
async fn search_form_posts_to_lang_path() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/fr?ip=45.77.77.77",
        vec![("User-Agent", "Mozilla/5.0")],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("action=\"/fr\""));
}

#[tokio::test]
async fn invalid_lang_tag_returns_404() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/xx?ip=45.77.77.77",
        vec![("User-Agent", "Mozilla/5.0")],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("404"));
}

// --- Dev mode tests ---

#[tokio::test]
async fn dev_mode_root_redirects_without_cf_header() {
    let app = build_test_app_with_dev_mode(true);
    let response = get_response(
        &app,
        "/",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn dev_mode_lang_path_returns_html_without_cf_header() {
    let app = build_test_app_with_dev_mode(true);
    let (status, body) = get_with_headers(
        &app,
        "/en",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("1.1.1.1"));
}

#[tokio::test]
async fn dev_mode_cli_returns_fallback_ip() {
    let app = build_test_app_with_dev_mode(true);
    let (status, body) = get_with_headers(&app, "/", vec![("User-Agent", "curl/8.7.1")]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "1.1.1.1");
}

#[tokio::test]
async fn dev_mode_json_returns_fallback_ip() {
    let app = build_test_app_with_dev_mode(true);
    let (status, body) = get(&app, "/json").await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"], "1.1.1.1");
}

#[tokio::test]
async fn dev_mode_cf_header_takes_precedence() {
    let app = build_test_app_with_dev_mode(true);
    let (status, body) = get_with_headers(
        &app,
        "/",
        vec![
            ("User-Agent", "curl/8.7.1"),
            ("CF-Connecting-IP", "8.8.8.8"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "8.8.8.8");
}

#[tokio::test]
async fn dev_mode_query_param_overrides_fallback() {
    let app = build_test_app_with_dev_mode(true);
    let (status, body) = get(&app, "/json?ip=45.77.77.77").await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"], "45.77.77.77");
}

#[tokio::test]
async fn dev_mode_field_endpoints_return_fallback_data() {
    let app = build_test_app_with_dev_mode(true);

    let (status, body) = get(&app, "/ip").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "1.1.1.1");

    let (status, body) = get(&app, "/asn").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "AS13335");

    let (status, body) = get(&app, "/org").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "Cloudflare, Inc.");
}

// --- Error page tests ---

#[tokio::test]
async fn not_found_returns_html_error_page() {
    let app = build_test_app();
    let (status, body) = get(&app, "/nonexistent-page").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("404"));
    assert!(body.contains("Not Found"));
    assert!(body.contains("Go to Home"));
}

#[tokio::test]
async fn browser_invalid_ip_shows_alert_html() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=not-an-ip",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("alert("));
    assert!(body.contains("not-an-ip"));
}

#[tokio::test]
async fn browser_non_public_ip_shows_alert_html() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=0.0.0.0",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("alert("));
}

#[tokio::test]
async fn browser_missing_client_ip_shows_alert_html() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("alert("));
}

#[tokio::test]
async fn cli_invalid_ip_returns_plain_text() {
    let app = build_test_app();
    let (status, body) = get(&app, "/json?ip=not-an-ip").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!body.contains("<!DOCTYPE html>"));
    assert_eq!(body, "Invalid IP address");
}

#[tokio::test]
async fn error_alert_page_has_search_form() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=bad-ip",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("<form"));
    assert!(body.contains("value=\"bad-ip\""));
    assert!(body.contains("autofocus"));
}

// --- Trailing slash tests ---

#[tokio::test]
async fn trailing_slash_redirects_to_canonical() {
    let app = build_test_app_with_dev_mode(true);
    let tags = [
        "en", "es", "de", "ja", "ko", "id", "fr", "ru", "pt", "it", "zh-Hant", "zh-Hans", "nl",
        "ar",
    ];
    for tag in tags {
        let uri = format!("/{tag}/");
        let response = get_response(&app, &uri, vec![("User-Agent", "Mozilla/5.0")]).await;
        assert_eq!(
            response.status(),
            StatusCode::PERMANENT_REDIRECT,
            "/{tag}/ should redirect with 308, got {}",
            response.status()
        );
        let location = response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(
            location,
            format!("/{tag}"),
            "/{tag}/ should redirect to /{tag}"
        );
    }
}

#[tokio::test]
async fn trailing_slash_preserves_query_params() {
    let app = build_test_app_with_dev_mode(true);
    let response = get_response(&app, "/de/?ip=1.2.3.4", vec![("User-Agent", "Mozilla/5.0")]).await;
    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(location, "/de?ip=1.2.3.4");
}

// --- i18n-specific tests ---

#[tokio::test]
async fn root_with_ip_query_redirects_with_ip() {
    let app = build_test_app();
    let response = get_response(
        &app,
        "/?ip=1.2.3.4",
        vec![
            ("User-Agent", "Mozilla/5.0"),
            ("CF-Connecting-IP", "1.1.1.1"),
        ],
    )
    .await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        location.contains("ip=1.2.3.4"),
        "redirect should contain ip param: {location}"
    );
}

#[tokio::test]
async fn all_locale_paths_return_200() {
    let app = build_test_app_with_dev_mode(true);
    let tags = [
        "en", "es", "de", "ja", "ko", "id", "fr", "ru", "pt", "it", "zh-Hant", "zh-Hans", "nl",
        "ar",
    ];
    for tag in tags {
        let uri = format!("/{tag}");
        let (status, body) =
            get_with_headers(&app, &uri, vec![("User-Agent", "Mozilla/5.0")]).await;
        assert_eq!(status, StatusCode::OK, "expected 200 for /{tag}");
        assert!(
            body.contains(&format!("lang=\"{tag}\"")),
            "/{tag} should have lang=\"{tag}\" attribute"
        );
    }
}

#[tokio::test]
async fn error_page_has_localized_go_home_link() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/nonexistent",
        vec![("User-Agent", "Mozilla/5.0"), ("Accept-Language", "en")],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("Go to Home"));
    assert!(body.contains("href=\"/en\""));
}

#[tokio::test]
async fn german_page_contains_translated_escaped_quote_strings() {
    let app = build_test_app_with_dev_mode(true);
    let (status, body) = get_with_headers(&app, "/de", vec![("User-Agent", "Mozilla/5.0")]).await;
    assert_eq!(status, StatusCode::OK);
    // These strings were previously untranslated due to escaped quotes in PO files.
    assert!(
        !body.contains("Rest assured, this service will definitely not"),
        "German page should not contain English Rug Pull string"
    );
    assert!(
        body.contains("Rug Pull"),
        "German page should still contain Rug Pull term"
    );
    assert!(
        !body.contains("Our API is only suitable for manual calls"),
        "German page should not contain English API string"
    );
    assert!(
        !body.contains("IP geographic data primarily comes from"),
        "German page should not contain English data source string"
    );
}

// --- JSON-LD and Microdata tests ---

#[tokio::test]
async fn html_contains_jsonld_on_landing_page() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en",
        vec![
            (
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            ),
            ("CF-Connecting-IP", "45.77.77.77"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("application/ld+json"),
        "landing page should contain JSON-LD script tag"
    );
    assert!(
        body.contains("\"@type\": \"WebApplication\""),
        "landing page should contain WebApplication JSON-LD"
    );
    assert!(
        body.contains("\"@type\": \"FAQPage\""),
        "landing page should contain FAQPage JSON-LD"
    );
    assert!(
        body.contains("\"applicationCategory\": \"UtilitiesApplication\""),
        "WebApplication should have applicationCategory"
    );
    assert!(
        body.contains("\"@type\": \"Question\""),
        "FAQPage should contain Question entities"
    );
    assert!(
        body.contains("\"@type\": \"Answer\""),
        "FAQPage should contain Answer entities"
    );
}

#[tokio::test]
async fn html_query_result_has_no_jsonld() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=45.77.77.77",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !body.contains("application/ld+json"),
        "query result page should not contain JSON-LD"
    );
}

#[tokio::test]
async fn html_contains_microdata_faq() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=45.77.77.77",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("itemtype=\"https://schema.org/FAQPage\""),
        "FAQ section should have FAQPage microdata"
    );
    assert!(
        body.contains("itemtype=\"https://schema.org/Question\""),
        "FAQ items should have Question microdata"
    );
    assert!(
        body.contains("itemtype=\"https://schema.org/Answer\""),
        "FAQ items should have Answer microdata"
    );
    assert!(
        body.contains("itemprop=\"name\""),
        "FAQ questions should have name itemprop"
    );
    assert!(
        body.contains("itemprop=\"acceptedAnswer\""),
        "FAQ questions should have acceptedAnswer itemprop"
    );
    assert!(
        body.contains("itemprop=\"text\""),
        "FAQ answers should have text itemprop"
    );
}

#[tokio::test]
async fn html_contains_microdata_ip_result() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=45.77.77.77",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("itemtype=\"https://schema.org/WebPage\""),
        "IP result section should have WebPage microdata"
    );
    assert!(
        body.contains("itemtype=\"https://schema.org/Organization\""),
        "Org row should have Organization microdata"
    );
    assert!(
        body.contains("itemprop=\"addressCountry\""),
        "Country row should have addressCountry itemprop"
    );
    assert!(
        body.contains("itemtype=\"https://schema.org/Place\""),
        "Location rows should have Place microdata"
    );
}

#[tokio::test]
async fn jsonld_contains_available_languages() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en",
        vec![
            (
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            ),
            ("CF-Connecting-IP", "45.77.77.77"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("\"availableLanguage\""),
        "WebApplication JSON-LD should list available languages"
    );
    assert!(
        body.contains("\"inLanguage\": \"en\""),
        "WebApplication JSON-LD should specify current language"
    );
}

// --- Non-public IP rejection tests ---

#[tokio::test]
async fn private_ipv4_rejected_on_all_endpoints() {
    let app = build_test_app();
    let private_ips = [
        "10.0.0.1",
        "172.16.0.1",
        "192.168.1.1",
        "127.0.0.1",
        "0.0.0.0",
        "255.255.255.255",
        "169.254.1.1",
        "100.64.0.1",
    ];
    let endpoints = [
        "/json", "/ip", "/asn", "/org", "/country", "/city", "/proxy", "/vpn", "/hosting", "/tor",
    ];

    for ip in private_ips {
        for endpoint in &endpoints {
            let uri = format!("{endpoint}?ip={ip}");
            let (status, body) = get(&app, &uri).await;
            assert_eq!(
                status,
                StatusCode::BAD_REQUEST,
                "expected 400 for {endpoint}?ip={ip}, got {status}"
            );
            assert_eq!(
                body, "Only public IP addresses can be queried",
                "wrong error message for {endpoint}?ip={ip}"
            );
        }
    }
}

#[tokio::test]
async fn private_ipv6_rejected() {
    let app = build_test_app();
    let private_ips = [
        "::1",
        "::",
        "fe80::1",
        "fc00::1",
        "fd00::1",
        "ff02::1",
        "2001:db8::1",
        "::ffff:127.0.0.1",
        "::ffff:192.168.1.1",
    ];
    for ip in private_ips {
        let uri = format!("/json?ip={ip}");
        let (status, body) = get(&app, &uri).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "expected 400 for /json?ip={ip}, got {status}"
        );
        assert_eq!(
            body, "Only public IP addresses can be queried",
            "wrong error message for /json?ip={ip}"
        );
    }
}

#[tokio::test]
async fn public_ips_still_accepted() {
    let app = build_test_app();
    let (status, _) = get(&app, "/json?ip=1.1.1.1").await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = get(&app, "/json?ip=2606:4700:4700::1111").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn private_ip_via_cf_header_still_works() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/",
        vec![
            ("User-Agent", "curl/8.7.1"),
            ("CF-Connecting-IP", "10.0.0.1"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "10.0.0.1");
}

#[tokio::test]
async fn browser_private_ipv4_shows_error() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=192.168.1.1",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("alert("));
    assert!(body.contains("192.168.1.1"));
}

#[tokio::test]
async fn browser_loopback_shows_error() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=127.0.0.1",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("alert("));
}

#[tokio::test]
async fn browser_broadcast_shows_error() {
    let app = build_test_app();
    let (status, body) = get_with_headers(
        &app,
        "/en?ip=255.255.255.255",
        vec![(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("alert("));
}
