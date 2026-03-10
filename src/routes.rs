use std::sync::Arc;

use axum::Router;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::get;
use tower::ServiceExt as _;

use crate::handlers::{self, AppState};
use crate::middleware::{ipv4_domain_security_headers, security_headers};
use crate::static_assets;

fn build_main_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::root_redirect))
        .route("/favicon.ico", get(handlers::favicon))
        .route("/site.webmanifest", get(handlers::manifest))
        .route("/health", get(handlers::health))
        .route("/openapi.json", get(handlers::openapi_json))
        .route("/robots.txt", get(handlers::robots_txt))
        .route("/sitemap.xml", get(handlers::sitemap_xml))
        .route("/json", get(handlers::json_handler))
        .route("/ip", get(handlers::ip_handler))
        .route("/asn", get(handlers::asn_handler))
        .route("/org", get(handlers::org_handler))
        .route("/country", get(handlers::country_handler))
        .route("/city", get(handlers::city_handler))
        .route("/proxy", get(handlers::proxy_handler))
        .route("/vpn", get(handlers::vpn_handler))
        .route("/hosting", get(handlers::hosting_handler))
        .route("/tor", get(handlers::tor_handler))
        .route("/static/{*path}", get(static_assets::serve_static))
        .route("/{lang}", get(handlers::root))
        .route("/{lang}/", get(handlers::root_trailing_slash))
        .fallback(handlers::not_found)
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

fn build_ipv4_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::ipv4_ip_handler))
        .route("/health", get(handlers::health))
        .fallback(handlers::ipv4_not_found)
        .layer(middleware::from_fn(ipv4_domain_security_headers))
        .with_state(state)
}

pub fn build_router(state: AppState) -> Router {
    let ipv4_domain: Arc<str> = state.ipv4_domain.clone();
    let main_router = build_main_router(state.clone());
    let ipv4_router = build_ipv4_router(state);

    let svc = tower::service_fn(move |req: axum::http::Request<axum::body::Body>| {
        let ipv4_domain = ipv4_domain.clone();
        let main_router = main_router.clone();
        let ipv4_router = ipv4_router.clone();
        async move {
            let is_ipv4_domain = !ipv4_domain.is_empty()
                && req
                    .headers()
                    .get("host")
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|host| {
                        let host = host.split(':').next().unwrap_or(host);
                        host.eq_ignore_ascii_case(&ipv4_domain)
                    });

            // Router<()> is an infallible service; unwrap is safe.
            let response = if is_ipv4_domain {
                ipv4_router.oneshot(req).await.into_response()
            } else {
                main_router.oneshot(req).await.into_response()
            };
            Ok::<_, std::convert::Infallible>(response)
        }
    });

    Router::new().fallback_service(svc)
}
