//! HTTP request handlers, split into sub-modules by concern.

mod api;
mod html;
mod seo;

use std::net::IpAddr;
use std::sync::Arc;

use axum::http::{HeaderMap, header};
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
pub use seo::{build_openapi_json, favicon, manifest, openapi_json, robots_txt, sitemap_xml};

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
    db::lookup(db, ip)
        .map_err(|_| AppError::DbLookupFailed)?
        .ok_or(AppError::IpNotFound)
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
            match extract_client_ip(headers, state.dev_mode).and_then(|ip| lookup_ip(&state.db, ip))
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
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(accepts_application_json)
}

fn accepts_application_json(accept: &str) -> bool {
    accept.split(',').any(|part| {
        let mut pieces = part.split(';');
        let media_type = pieces.next().unwrap_or_default().trim();
        if !media_type.eq_ignore_ascii_case("application/json") {
            return false;
        }

        let mut quality = 1.0;
        for param in pieces {
            let param = param.trim();
            if let Some((name, value)) = param.split_once('=')
                && name.trim().eq_ignore_ascii_case("q")
            {
                quality = value
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|q| (0.0..=1.0).contains(q))
                    .unwrap_or(0.0);
            }
        }
        quality > 0.0
    })
}

fn append_ip_query(path: &mut String, query: &IpQuery) {
    if let Some(ref ip) = query.ip
        && !ip.trim().is_empty()
    {
        path.push_str("?ip=");
        path.push_str(&urlencoding::encode(ip));
    }
}
