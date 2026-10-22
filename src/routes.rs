//! Axum router construction with host-based routing for IPv4 and main domains.

use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, header};
use axum::response::Response;
use axum::routing::get;
use tower::Service;
use tower::util::MapResponseLayer;

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
        .layer(MapResponseLayer::new(security_headers))
        .with_state(state)
}

fn build_ipv4_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::ipv4_ip_handler))
        .route("/health", get(handlers::health))
        .fallback(handlers::ipv4_not_found)
        .layer(MapResponseLayer::new(ipv4_domain_security_headers))
        .with_state(state)
}

/// Dispatches each request to the IPv4-only router or the main router based
/// on its `Host` header. Implemented as a plain service rather than an outer
/// `Router` so requests skip a second round of route matching and boxing.
#[derive(Clone)]
pub struct HostRouter {
    ipv4_domain: Arc<str>,
    main_router: Router,
    ipv4_router: Router,
}

impl HostRouter {
    fn is_ipv4_domain<B>(&self, req: &Request<B>) -> bool {
        !self.ipv4_domain.is_empty()
            && req
                .headers()
                .get(header::HOST)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|host| {
                    let host = host.split(':').next().unwrap_or(host);
                    host.eq_ignore_ascii_case(&self.ipv4_domain)
                })
    }
}

impl Service<Request<Body>> for HostRouter {
    type Response = Response;
    type Error = Infallible;
    type Future = <Router as Service<Request<Body>>>::Future;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Router<()> is always ready.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        if self.is_ipv4_domain(&req) {
            self.ipv4_router.call(req)
        } else {
            self.main_router.call(req)
        }
    }
}

pub fn build_router(state: AppState) -> HostRouter {
    HostRouter {
        ipv4_domain: state.ipv4_domain.clone(),
        main_router: build_main_router(state.clone()),
        ipv4_router: build_ipv4_router(state),
    }
}
