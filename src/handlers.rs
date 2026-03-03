use std::net::IpAddr;
use std::sync::Arc;

use askama::Template;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};

use crate::cli_detect::is_cli_user_agent;
use crate::db::{self, SharedDb};
use crate::error::AppError;
use crate::models::{IpInfo, get_en_name};
use crate::static_assets;

#[derive(serde::Deserialize)]
pub struct IpQuery {
    pub ip: Option<String>,
}

const DEV_FALLBACK_IP: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1));

#[derive(Clone)]
pub struct AppState {
    pub db: SharedDb,
    pub site_domain: Arc<str>,
    pub dev_mode: bool,
    pub openapi_json: Arc<str>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    ip: String,
    query: String,
    site_domain: Arc<str>,
    css_version: u64,
    asn: String,
    org: String,
    country: String,
    city: String,
    is_proxy: bool,
    is_vpn: bool,
    is_hosting: bool,
    is_tor: bool,
}

#[derive(Template)]
#[template(path = "error_alert.html")]
struct ErrorAlertTemplate {
    site_domain: Arc<str>,
    css_version: u64,
    query: String,
    error_message: String,
}

#[derive(Template)]
#[template(path = "error_page.html")]
struct ErrorPageTemplate {
    site_domain: Arc<str>,
    css_version: u64,
    status_code: u16,
    status_text: String,
    error_message: String,
}

fn resolve_ip(headers: &HeaderMap, query: &IpQuery, dev_mode: bool) -> Result<IpAddr, AppError> {
    if let Some(ref ip_str) = query.ip {
        let trimmed = ip_str.trim();
        if !trimmed.is_empty() {
            return trimmed.parse().map_err(|_| AppError::InvalidIp);
        }
    }
    extract_client_ip(headers, dev_mode)
}

fn extract_client_ip(headers: &HeaderMap, dev_mode: bool) -> Result<IpAddr, AppError> {
    if let Some(ip) = headers
        .get("CF-Connecting-IP")
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

fn render_error_alert(state: &AppState, query: &str, error: &AppError) -> Response {
    let template = ErrorAlertTemplate {
        site_domain: state.site_domain.clone(),
        css_version: static_assets::asset_version("style.css"),
        query: query.to_owned(),
        error_message: error.message().to_owned(),
    };
    let status = error.status_code();
    match template.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(_) => (status, error.message()).into_response(),
    }
}

fn render_error_page(site_domain: &Arc<str>, status: StatusCode, message: &str) -> Response {
    let template = ErrorPageTemplate {
        site_domain: site_domain.clone(),
        css_version: static_assets::asset_version("style.css"),
        status_code: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("Error").to_owned(),
        error_message: message.to_owned(),
    };
    match template.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(_) => (status, message.to_owned()).into_response(),
    }
}

pub async fn root(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Response {
    let ua = headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if is_cli_user_agent(ua) && query.ip.is_none() {
        return match extract_client_ip(&headers, state.dev_mode) {
            Ok(ip) => format!("{ip}\n").into_response(),
            Err(e) => e.into_response(),
        };
    }

    if wants_json(&headers) && query.ip.is_none() {
        return match extract_client_ip(&headers, state.dev_mode)
            .and_then(|ip| lookup_ip(&state.db, ip))
        {
            Ok(info) => axum::Json(info).into_response(),
            Err(e) => e.into_response(),
        };
    }

    let query_str = query.ip.clone().unwrap_or_default();

    let ip = match resolve_ip(&headers, &query, state.dev_mode) {
        Ok(ip) => ip,
        Err(e) => return render_error_alert(&state, &query_str, &e),
    };

    let info = match lookup_ip(&state.db, ip) {
        Ok(info) => info,
        Err(e) => return render_error_alert(&state, &query_str, &e),
    };

    let template = IndexTemplate {
        ip: info.ip.clone(),
        query: query.ip.unwrap_or_default(),
        site_domain: state.site_domain.clone(),
        css_version: static_assets::asset_version("style.css"),
        asn: format_asn(&info),
        org: format_org(&info).to_owned(),
        country: format_country(&info).to_owned(),
        city: format_city(&info).to_owned(),
        is_proxy: info.proxy.is_proxy,
        is_vpn: info.proxy.is_vpn,
        is_hosting: info.proxy.is_hosting,
        is_tor: info.proxy.is_tor,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => render_error_alert(&state, &query_str, &AppError::TemplateRenderFailed),
    }
}

pub async fn health() -> StatusCode {
    StatusCode::OK
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

pub async fn asn_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query, state.dev_mode)?;
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
    let ip = resolve_ip(&headers, &query, state.dev_mode)?;
    let reader = state.db.load();
    let org = db::lookup_asn_org(&reader, ip).unwrap_or_default();
    Ok(format!("{org}\n"))
}

pub async fn country_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query, state.dev_mode)?;
    let reader = state.db.load();
    let country = db::lookup_country_name(&reader, ip).unwrap_or_default();
    Ok(format!("{country}\n"))
}

pub async fn city_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query, state.dev_mode)?;
    let reader = state.db.load();
    let city = db::lookup_city_name(&reader, ip).unwrap_or_default();
    Ok(format!("{city}\n"))
}

pub async fn proxy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query, state.dev_mode)?;
    let reader = state.db.load();
    let proxy = db::lookup_proxy(&reader, ip).ok_or(AppError::DbLookupFailed)?;
    Ok(format!("{}\n", proxy.is_proxy))
}

pub async fn vpn_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query, state.dev_mode)?;
    let reader = state.db.load();
    let proxy = db::lookup_proxy(&reader, ip).ok_or(AppError::DbLookupFailed)?;
    Ok(format!("{}\n", proxy.is_vpn))
}

pub async fn hosting_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query, state.dev_mode)?;
    let reader = state.db.load();
    let proxy = db::lookup_proxy(&reader, ip).ok_or(AppError::DbLookupFailed)?;
    Ok(format!("{}\n", proxy.is_hosting))
}

pub async fn tor_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Result<String, AppError> {
    let ip = resolve_ip(&headers, &query, state.dev_mode)?;
    let reader = state.db.load();
    let proxy = db::lookup_proxy(&reader, ip).ok_or(AppError::DbLookupFailed)?;
    Ok(format!("{}\n", proxy.is_tor))
}

pub async fn openapi_json(State(state): State<AppState>) -> Response {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        String::from(state.openapi_json.as_ref()),
    )
        .into_response()
}

pub fn build_openapi_json(site_domain: &str) -> Arc<str> {
    let mut spec: serde_json::Value =
        serde_json::from_str(include_str!("../openapi.json")).expect("openapi.json must be valid");

    let server_url = format!("https://{site_domain}");
    spec["servers"] = serde_json::json!([{ "url": server_url }]);

    let json = serde_json::to_string_pretty(&spec).expect("serialization cannot fail");
    Arc::from(json)
}

pub async fn robots_txt(State(state): State<AppState>) -> Response {
    let body = format!(
        "User-agent: *\n\
         Allow: /\n\
         Disallow: /json\n\
         Disallow: /ip\n\
         Disallow: /asn\n\
         Disallow: /org\n\
         Disallow: /country\n\
         Disallow: /city\n\
         Disallow: /proxy\n\
         Disallow: /vpn\n\
         Disallow: /hosting\n\
         Disallow: /tor\n\
         Disallow: /health\n\
         Disallow: /openapi.json\n\
         \n\
         Sitemap: https://{}/sitemap.xml\n",
        state.site_domain
    );
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body).into_response()
}

pub async fn sitemap_xml(State(state): State<AppState>) -> Response {
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         \x20 <url>\n\
         \x20   <loc>https://{}/</loc>\n\
         \x20 </url>\n\
         </urlset>\n",
        state.site_domain
    );
    (
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        body,
    )
        .into_response()
}

pub async fn not_found(State(state): State<AppState>) -> Response {
    render_error_page(
        &state.site_domain,
        StatusCode::NOT_FOUND,
        "The page you are looking for does not exist.",
    )
}
