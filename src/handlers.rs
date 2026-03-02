use std::net::IpAddr;
use std::sync::Arc;

use askama::Template;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

use crate::cli_detect::is_cli_user_agent;
use crate::db::{self, SharedDb};
use crate::error::AppError;
use crate::models::{IpInfo, get_en_name};

#[derive(serde::Deserialize)]
pub struct IpQuery {
    pub ip: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: SharedDb,
    pub site_domain: Arc<str>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    ip: String,
    query: String,
    site_domain: Arc<str>,
    asn: String,
    org: String,
    country: String,
    city: String,
    is_proxy: bool,
    is_vpn: bool,
    is_hosting: bool,
    is_tor: bool,
}

fn resolve_ip(headers: &HeaderMap, query: &IpQuery) -> Result<IpAddr, AppError> {
    if let Some(ref ip_str) = query.ip {
        let trimmed = ip_str.trim();
        if !trimmed.is_empty() {
            return trimmed.parse().map_err(|_| AppError::InvalidIp);
        }
    }
    extract_client_ip(headers)
}

fn extract_client_ip(headers: &HeaderMap) -> Result<IpAddr, AppError> {
    headers
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse().ok())
        .ok_or(AppError::MissingClientIp)
}

fn lookup_ip(db: &SharedDb, ip: IpAddr) -> Result<IpInfo, AppError> {
    let reader = db.load();
    db::lookup(&reader, ip).ok_or(AppError::DbLookupFailed)
}

fn wants_json(headers: &HeaderMap) -> bool {
    headers
        .get("Accept")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/json"))
}

fn format_asn(info: &IpInfo) -> String {
    info.asn
        .as_ref()
        .and_then(|a| a.autonomous_system_number)
        .map(|n| format!("AS{n}"))
        .unwrap_or_default()
}

fn format_org(info: &IpInfo) -> &str {
    info.asn
        .as_ref()
        .and_then(|a| a.autonomous_system_organization.as_deref())
        .unwrap_or_default()
}

fn format_country(info: &IpInfo) -> &str {
    info.country
        .as_ref()
        .map(|c| get_en_name(&c.names))
        .unwrap_or_default()
}

fn format_city(info: &IpInfo) -> &str {
    info.city
        .as_ref()
        .map(|c| get_en_name(&c.names))
        .unwrap_or_default()
}

pub async fn root(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<Response, AppError> {
    let ua = headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if is_cli_user_agent(ua) && query.ip.is_none() {
        let ip = extract_client_ip(&headers)?;
        return Ok(format!("{ip}\n").into_response());
    }

    if wants_json(&headers) && query.ip.is_none() {
        let ip = extract_client_ip(&headers)?;
        let info = lookup_ip(&state.db, ip)?;
        return Ok(axum::Json(info).into_response());
    }

    let ip = resolve_ip(&headers, &query)?;
    let info = lookup_ip(&state.db, ip)?;

    let template = IndexTemplate {
        ip: info.ip.clone(),
        query: query.ip.unwrap_or_default(),
        site_domain: state.site_domain.clone(),
        asn: format_asn(&info),
        org: format_org(&info).to_owned(),
        country: format_country(&info).to_owned(),
        city: format_city(&info).to_owned(),
        is_proxy: info.proxy.is_proxy,
        is_vpn: info.proxy.is_vpn,
        is_hosting: info.proxy.is_hosting,
        is_tor: info.proxy.is_tor,
    };

    Ok(Html(template.render().map_err(|_| AppError::DbLookupFailed)?).into_response())
}

pub async fn health() -> StatusCode {
    StatusCode::OK
}

pub async fn json_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<axum::Json<IpInfo>, AppError> {
    let ip = resolve_ip(&headers, &query)?;
    let info = lookup_ip(&state.db, ip)?;
    Ok(axum::Json(info))
}

pub async fn ip_handler(
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query)?;
    Ok(format!("{ip}\n"))
}

pub async fn asn_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query)?;
    let reader = state.db.load();
    let text = db::lookup_asn_number(&reader, ip)
        .map(|n| format!("AS{n}"))
        .unwrap_or_default();
    Ok(format!("{text}\n"))
}

pub async fn org_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query)?;
    let reader = state.db.load();
    let org = db::lookup_asn_org(&reader, ip).unwrap_or_default();
    Ok(format!("{org}\n"))
}

pub async fn country_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query)?;
    let reader = state.db.load();
    let country = db::lookup_country_name(&reader, ip).unwrap_or_default();
    Ok(format!("{country}\n"))
}

pub async fn city_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query)?;
    let reader = state.db.load();
    let city = db::lookup_city_name(&reader, ip).unwrap_or_default();
    Ok(format!("{city}\n"))
}

pub async fn proxy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query)?;
    let reader = state.db.load();
    let proxy = db::lookup_proxy(&reader, ip).ok_or(AppError::DbLookupFailed)?;
    Ok(format!("{}\n", proxy.is_proxy))
}

pub async fn vpn_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query)?;
    let reader = state.db.load();
    let proxy = db::lookup_proxy(&reader, ip).ok_or(AppError::DbLookupFailed)?;
    Ok(format!("{}\n", proxy.is_vpn))
}

pub async fn hosting_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query)?;
    let reader = state.db.load();
    let proxy = db::lookup_proxy(&reader, ip).ok_or(AppError::DbLookupFailed)?;
    Ok(format!("{}\n", proxy.is_hosting))
}

pub async fn tor_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query)?;
    let reader = state.db.load();
    let proxy = db::lookup_proxy(&reader, ip).ok_or(AppError::DbLookupFailed)?;
    Ok(format!("{}\n", proxy.is_tor))
}
