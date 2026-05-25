use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};

use crate::db;
use crate::error::AppError;
use crate::i18n::{self, Locale};
use crate::models::PageInfo;
use crate::static_assets;

use super::{AppState, IpQuery, append_ip_query, handle_non_browser_request, resolve_ip};

/// Askama custom filters. The `filters` module name is required by Askama's
/// filter resolution. Usage in templates: `{{ "msgid"|t(locale) }}`.
mod filters {
    use crate::i18n::{self, Locale};

    #[askama::filter_fn]
    pub fn t<'a>(
        msgid: &'a str,
        _env: &dyn askama::Values,
        locale: &Locale,
    ) -> askama::Result<&'a str> {
        Ok(i18n::translate(*locale, msgid))
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    ip: String,
    query: String,
    is_query: bool,
    site_domain: Arc<str>,
    ipv4_domain: Arc<str>,
    css_version: u64,
    js_version: u64,
    asn: String,
    asn_number: Option<u32>,
    org: String,
    country: String,
    country_code: String,
    city: String,
    is_proxy: bool,
    is_vpn: bool,
    is_hosting: bool,
    is_tor: bool,
    locale: Locale,
    lang_tag: &'static str,
    html_dir: &'static str,
    all_locales: &'static [Locale],
}

#[derive(Template)]
#[template(path = "error_alert.html")]
struct ErrorAlertTemplate {
    site_domain: Arc<str>,
    css_version: u64,
    js_version: u64,
    query: String,
    error_message: String,
    locale: Locale,
    lang_tag: &'static str,
    html_dir: &'static str,
    all_locales: &'static [Locale],
}

#[derive(Template)]
#[template(path = "error_page.html")]
struct ErrorPageTemplate {
    site_domain: Arc<str>,
    css_version: u64,
    status_code: u16,
    status_text: String,
    error_message: String,
    lang_tag: &'static str,
    html_dir: &'static str,
    locale: Locale,
}

fn lookup_page_info(
    db: &db::SharedDb,
    ip: std::net::IpAddr,
    locale: Locale,
) -> Result<PageInfo, AppError> {
    match db::lookup_page_info(db, ip, locale.mmdb_key()).map_err(|_| AppError::DbLookupFailed)? {
        Some(info) => Ok(info),
        None if !db::is_ready(db) => Err(AppError::DbNotReady),
        None => Err(AppError::IpNotFound),
    }
}

fn render_error_alert(state: &AppState, locale: Locale, query: &str, error: &AppError) -> Response {
    let error_msg = i18n::translate(locale, error.message());
    let template = ErrorAlertTemplate {
        site_domain: state.site_domain.clone(),
        css_version: static_assets::asset_version("style.css"),
        js_version: static_assets::asset_version("script.js"),
        query: query.to_owned(),
        error_message: error_msg.to_owned(),
        locale,
        lang_tag: locale.tag(),
        html_dir: locale.html_dir(),
        all_locales: Locale::ALL,
    };
    let status = error.status_code();
    match template.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(_) => (status, error.message()).into_response(),
    }
}

fn render_error_page(
    site_domain: &Arc<str>,
    locale: Locale,
    status: StatusCode,
    message: &str,
) -> Response {
    let localized_message = i18n::translate(locale, message);
    let template = ErrorPageTemplate {
        site_domain: site_domain.clone(),
        css_version: static_assets::asset_version("style.css"),
        status_code: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("Error").to_owned(),
        error_message: localized_message.to_owned(),
        lang_tag: locale.tag(),
        html_dir: locale.html_dir(),
        locale,
    };
    match template.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(_) => (status, message.to_owned()).into_response(),
    }
}

pub async fn root_redirect(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Response {
    // CLI clients and JSON requests bypass i18n entirely.
    if let Some(response) = handle_non_browser_request(&state, &headers, &query) {
        return response;
    }

    let locale = i18n::negotiate_locale(&headers);
    let mut redirect_path = format!("/{}", locale.tag());
    append_ip_query(&mut redirect_path, &query);

    Redirect::temporary(&redirect_path).into_response()
}

pub async fn root_trailing_slash(
    Path(lang): Path<String>,
    Query(query): Query<IpQuery>,
) -> Response {
    let mut redirect_path = format!("/{lang}");
    append_ip_query(&mut redirect_path, &query);
    Redirect::permanent(&redirect_path).into_response()
}

pub async fn root(
    State(state): State<AppState>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    Query(query): Query<IpQuery>,
) -> Response {
    let locale = match Locale::from_tag(&lang) {
        Some(l) => l,
        None => {
            return render_error_page(
                &state.site_domain,
                Locale::En,
                StatusCode::NOT_FOUND,
                "The page you are looking for does not exist.",
            );
        }
    };

    if let Some(response) = handle_non_browser_request(&state, &headers, &query) {
        return response;
    }

    if let Some(ref ip_str) = query.ip
        && ip_str.trim().is_empty()
    {
        return Redirect::temporary(&format!("/{}", locale.tag())).into_response();
    }

    let query_str = query.ip.clone().unwrap_or_default();

    let ip = match resolve_ip(&headers, &query, state.dev_mode) {
        Ok(ip) => ip,
        Err(e) => return render_error_alert(&state, locale, &query_str, &e),
    };

    let info = match lookup_page_info(&state.db, ip, locale) {
        Ok(info) => info,
        Err(e) => return render_error_alert(&state, locale, &query_str, &e),
    };

    let is_query = query.ip.is_some();
    let template = IndexTemplate {
        ip: info.ip,
        query: query.ip.unwrap_or_default(),
        is_query,
        site_domain: state.site_domain.clone(),
        ipv4_domain: state.ipv4_domain.clone(),
        css_version: static_assets::asset_version("style.css"),
        js_version: static_assets::asset_version("script.js"),
        asn: info.asn,
        asn_number: info.asn_number,
        org: info.org,
        country: info.country,
        country_code: info.country_code,
        city: info.city,
        is_proxy: info.proxy.is_proxy,
        is_vpn: info.proxy.is_vpn,
        is_hosting: info.proxy.is_hosting,
        is_tor: info.proxy.is_tor,
        locale,
        lang_tag: locale.tag(),
        html_dir: locale.html_dir(),
        all_locales: Locale::ALL,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => render_error_alert(&state, locale, &query_str, &AppError::TemplateRenderFailed),
    }
}

pub async fn not_found(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let locale = i18n::negotiate_locale(&headers);
    render_error_page(
        &state.site_domain,
        locale,
        StatusCode::NOT_FOUND,
        "The page you are looking for does not exist.",
    )
}
