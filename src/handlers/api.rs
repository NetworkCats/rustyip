use std::net::IpAddr;

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::db::{self, SharedDb};
use crate::error::AppError;
use crate::models::IpInfo;

use super::{AppState, IpQuery, extract_ip_from_header, lookup_ip, resolve_ip};

pub async fn health(State(state): State<AppState>) -> StatusCode {
    if db::is_ready(&state.db) {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

pub async fn ipv4_ip_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let ip = extract_ip_from_header(&headers, "X-IPv4-Client-IP", state.dev_mode)?;
    let origin = format!("https://{}", state.site_domain);
    Ok((
        [
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, origin),
            (header::ACCESS_CONTROL_ALLOW_METHODS, "GET".to_owned()),
            (
                header::HeaderName::from_static("access-control-max-age"),
                "86400".to_owned(),
            ),
        ],
        format!("{ip}\n"),
    )
        .into_response())
}

pub async fn ipv4_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

pub async fn json_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<axum::Json<IpInfo>, AppError> {
    let ip = resolve_ip(&headers, &query, state.dev_mode)?;
    let info = lookup_ip(&state.db, ip)?;
    Ok(axum::Json(info))
}

pub async fn ip_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query, state.dev_mode)?;
    Ok(format!("{ip}\n"))
}

fn field_handler<T: std::fmt::Display + Default>(
    state: &AppState,
    headers: &HeaderMap,
    query: &IpQuery,
    lookup: impl FnOnce(&SharedDb, IpAddr) -> Option<T>,
) -> Result<String, AppError> {
    if !db::is_ready(&state.db) {
        return Err(AppError::DbNotReady);
    }
    let ip = resolve_ip(headers, query, state.dev_mode)?;
    let value = lookup(&state.db, ip).unwrap_or_default();
    Ok(format!("{value}\n"))
}

fn proxy_field_handler(
    state: &AppState,
    headers: &HeaderMap,
    query: &IpQuery,
    field: impl FnOnce(&crate::models::ProxyInfo) -> bool,
) -> Result<String, AppError> {
    if !db::is_ready(&state.db) {
        return Err(AppError::DbNotReady);
    }
    let ip = resolve_ip(headers, query, state.dev_mode)?;
    let value = db::lookup_proxy(&state.db, ip).is_some_and(|p| field(&p));
    Ok(format!("{value}\n"))
}

pub async fn asn_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    field_handler(&state, &headers, &query, |db, ip| {
        db::lookup_asn_number(db, ip).map(|n| format!("AS{n}"))
    })
}

pub async fn org_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    field_handler(&state, &headers, &query, db::lookup_asn_org)
}

pub async fn country_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    field_handler(&state, &headers, &query, db::lookup_country_name)
}

pub async fn city_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    field_handler(&state, &headers, &query, db::lookup_city_name)
}

pub async fn proxy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    proxy_field_handler(&state, &headers, &query, |p| p.is_proxy)
}

pub async fn vpn_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    proxy_field_handler(&state, &headers, &query, |p| p.is_vpn)
}

pub async fn hosting_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    proxy_field_handler(&state, &headers, &query, |p| p.is_hosting)
}

pub async fn tor_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    proxy_field_handler(&state, &headers, &query, |p| p.is_tor)
}
