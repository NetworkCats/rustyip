use axum::Router;
use axum::middleware;
use axum::routing::get;

use crate::handlers::{self, AppState};
use crate::middleware::security_headers;
use crate::static_assets;

pub fn build_router(state: AppState) -> Router {
    let localized = Router::new()
        .route("/", get(handlers::root))
        .with_state(state.clone());

    Router::new()
        .route("/", get(handlers::root_redirect))
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
        .nest("/{lang}", localized)
        .fallback(handlers::not_found)
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}
