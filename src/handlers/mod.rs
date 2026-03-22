//! HTTP request handlers, split into sub-modules by concern.

mod api;
mod html;
mod seo;

use std::net::IpAddr;
use std::sync::Arc;

use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};

use crate::db::{self, SharedDb};
use crate::error::AppError;
use crate::ip_validate;
use crate::models::IpInfo;
use crate::ua_detect::is_plain_text_agent;

// Re-export all public handlers for use by routes.rs
pub use api::{
    asn_handler, city_handler, country_handler, health, hosting_handler, ip_handler,
    ipv4_ip_handler, ipv4_not_found, json_handler, org_handler, proxy_handler, tor_handler,
    vpn_handler,
};
pub use html::{not_found, root, root_redirect, root_trailing_slash};
pub use seo::{
    build_openapi_json, favicon, manifest, openapi_json, robots_txt, sitemap_xml,
};

#[derive(serde::Deserialize)]
pub struct IpQuery {
    pub ip: Option<String>,
}

const DEV_FALLBACK_IP: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1));

#[derive(Clone)]
pub struct AppState {
    pub db: SharedDb,
    pub site_domain: Arc<str>,
    pub ipv4_domain: Arc<str>,
    pub dev_mode: bool,
    pub openapi_json: Arc<str>,
}

fn resolve_ip(headers: &HeaderMap, query: &IpQuery, dev_mode: bool) -> Result<IpAddr, AppError> {
    if let Some(ref ip_str) = query.ip {
        let trimmed = ip_str.trim();
        if !trimmed.is_empty() {
            let ip: IpAddr = trimmed.parse().map_err(|_| AppError::InvalidIp)?;
            if !ip_validate::is_global_ip(ip) {
                return Err(AppError::NonPublicIp);
            }
            return Ok(ip);
        }
    }
    extract_client_ip(headers, dev_mode)
}

fn extract_ip_from_header(
    headers: &HeaderMap,
    header_name: &str,
    dev_mode: bool,
) -> Result<IpAddr, AppError> {
    if let Some(ip) = headers
        .get(header_name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse().ok())
    {
        return Ok(ip);
    }

    if dev_mode {
        return Ok(DEV_FALLBACK_IP);
    }

    Err(AppError::MissingClientIp)
}

fn extract_client_ip(headers: &HeaderMap, dev_mode: bool) -> Result<IpAddr, AppError> {
    extract_ip_from_header(headers, "CF-Connecting-IP", dev_mode)
}

fn lookup_ip(db: &SharedDb, ip: IpAddr) -> Result<IpInfo, AppError> {
    if !db::is_ready(db) {
        return Err(AppError::DbNotReady);
    }
    db::lookup(db, ip).ok_or(AppError::DbLookupFailed)
}

fn handle_non_browser_request(
    state: &AppState,
    headers: &HeaderMap,
    query: &IpQuery,
) -> Option<Response> {
    let ua = headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if is_plain_text_agent(ua) && query.ip.is_none() {
        return Some(match extract_client_ip(headers, state.dev_mode) {
            Ok(ip) => format!("{ip}\n").into_response(),
            Err(e) => e.into_response(),
        });
    }

    if wants_json(headers) && query.ip.is_none() {
        return Some(
            match extract_client_ip(headers, state.dev_mode)
                .and_then(|ip| lookup_ip(&state.db, ip))
            {
                Ok(info) => axum::Json(info).into_response(),
                Err(e) => e.into_response(),
            },
        );
    }

    None
}

fn wants_json(headers: &HeaderMap) -> bool {
    headers
        .get("Accept")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/json"))
}

fn append_ip_query(path: &mut String, query: &IpQuery) {
    if let Some(ref ip) = query.ip
        && !ip.trim().is_empty()
    {
        path.push_str("?ip=");
        path.push_str(&urlencoding::encode(ip));
    }
}
