use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;

use crate::handlers::{self, AppState};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::root))
        .route("/health", get(handlers::health))
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
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}
