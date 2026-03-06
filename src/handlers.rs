use std::net::IpAddr;
use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};

use crate::ua_detect::is_plain_text_agent;
use crate::db::{self, SharedDb};
use crate::error::AppError;
use crate::i18n::{self, Locale};
use crate::ip_validate;
use crate::models::{IpInfo, get_localized_name};
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
    is_query: bool,
    site_domain: Arc<str>,
    css_version: u64,
    asn: String,
    asn_number: Option<u32>,
    org: String,
    country: String,
    city: String,
    is_proxy: bool,
    is_vpn: bool,
    is_hosting: bool,
    is_tor: bool,
    locale: Locale,
    lang_tag: &'static str,
    html_dir: &'static str,
    t_title: &'static str,
    t_description: &'static str,
    t_search_label: &'static str,
    t_search_placeholder: &'static str,
    t_invalid_ip: &'static str,
    t_non_public_ip: &'static str,
    t_asn: &'static str,
    t_org: &'static str,
    t_country: &'static str,
    t_city: &'static str,
    t_proxy: &'static str,
    t_vpn: &'static str,
    t_hosting: &'static str,
    t_tor: &'static str,
    t_yes: &'static str,
    t_no: &'static str,
    t_faq: &'static str,
    t_faq_cli_q: &'static str,
    t_faq_cli_a: &'static str,
    t_faq_cli_query_current: &'static str,
    t_faq_cli_query_own: &'static str,
    t_faq_cli_json: &'static str,
    t_faq_cli_specific: &'static str,
    t_faq_ipv46_q: &'static str,
    t_faq_ipv46_a: &'static str,
    t_faq_json_q: &'static str,
    t_faq_json_a: &'static str,
    t_faq_automate_q: &'static str,
    t_faq_automate_a1: &'static str,
    t_faq_automate_a2: &'static str,
    t_faq_data_q: &'static str,
    t_faq_data_a: &'static str,
    t_faq_data_attr: &'static str,
    t_faq_selfhost_q: &'static str,
    t_faq_selfhost_a: &'static str,
    t_faq_diff_q: &'static str,
    t_faq_diff_a1: &'static str,
    t_faq_diff_a2: &'static str,
    t_faq_diff_a3: &'static str,
    t_faq_stable_q: &'static str,
    t_faq_stable_a1: &'static str,
    t_faq_stable_a2: &'static str,
    t_faq_stable_a3: &'static str,
    t_faq_sponsor_q: &'static str,
    t_faq_sponsor_a: &'static str,
    t_footer_license: &'static str,
    t_footer_source: &'static str,
    t_footer_db: &'static str,
    t_footer_attribution: &'static str,
    t_table_aria: &'static str,
    t_faq_aria: &'static str,
    t_nav_aria: &'static str,
    t_language: &'static str,
    all_locales: &'static [Locale],
}

#[derive(Template)]
#[template(path = "error_alert.html")]
struct ErrorAlertTemplate {
    site_domain: Arc<str>,
    css_version: u64,
    query: String,
    error_message: String,
    locale: Locale,
    lang_tag: &'static str,
    html_dir: &'static str,
    t_error: &'static str,
    t_search_label: &'static str,
    t_search_placeholder: &'static str,
    t_invalid_ip: &'static str,
    t_non_public_ip: &'static str,
    t_footer_license: &'static str,
    t_footer_source: &'static str,
    t_footer_db: &'static str,
    t_nav_aria: &'static str,
    t_language: &'static str,
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
    t_go_home: &'static str,
    locale: Locale,
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

fn asn_number(info: &IpInfo) -> Option<u32> {
    info.asn.as_ref().and_then(|a| a.autonomous_system_number)
}

fn format_asn(info: &IpInfo) -> String {
    asn_number(info)
        .map(|n| format!("AS{n}"))
        .unwrap_or_default()
}

fn format_org(info: &IpInfo) -> &str {
    info.asn
        .as_ref()
        .and_then(|a| a.autonomous_system_organization.as_deref())
        .unwrap_or_default()
}

fn format_country(info: &IpInfo, locale: Locale) -> &str {
    info.country
        .as_ref()
        .map(|c| get_localized_name(&c.names, locale.mmdb_key()))
        .unwrap_or_default()
}

fn format_city(info: &IpInfo, locale: Locale) -> &str {
    info.city
        .as_ref()
        .map(|c| get_localized_name(&c.names, locale.mmdb_key()))
        .unwrap_or_default()
}

fn build_translations(locale: Locale) -> Translations {
    Translations {
        t_title: i18n::translate(locale, "Accurate IP proxy information and geographic location"),
        t_description: i18n::translate(
            locale,
            "Find your public IP address instantly. Look up geolocation, ASN, organization, and proxy/VPN/Tor detection for any IP address.",
        ),
        t_search_label: i18n::translate(locale, "Search IP address"),
        t_search_placeholder: i18n::translate(locale, "Type to search IP data"),
        t_invalid_ip: i18n::translate(locale, "Invalid IP address"),
        t_non_public_ip: i18n::translate(locale, "Only public IP addresses can be queried"),
        t_asn: i18n::translate(locale, "ASN"),
        t_org: i18n::translate(locale, "Org"),
        t_country: i18n::translate(locale, "Country"),
        t_city: i18n::translate(locale, "City"),
        t_proxy: i18n::translate(locale, "Proxy"),
        t_vpn: i18n::translate(locale, "VPN"),
        t_hosting: i18n::translate(locale, "Hosting"),
        t_tor: i18n::translate(locale, "Tor"),
        t_yes: i18n::translate(locale, "Yes"),
        t_no: i18n::translate(locale, "No"),
        t_faq: i18n::translate(locale, "FAQ"),
        t_faq_cli_q: i18n::translate(locale, "How to use this service from the command line"),
        t_faq_cli_a: i18n::translate(
            locale,
            "You can use curl, httpie, wget, or whatever command line tool you prefer. Here's an example with curl:",
        ),
        t_faq_cli_query_current: i18n::translate(locale, "Check your current IP:"),
        t_faq_cli_query_own: i18n::translate(locale, "Look up information about your own IP:"),
        t_faq_cli_json: i18n::translate(locale, "Or get everything in JSON format:"),
        t_faq_cli_specific: i18n::translate(locale, "Look up information about a specific IP:"),
        t_faq_ipv46_q: i18n::translate(
            locale,
            "How do I force IPv4 or IPv6?",
        ),
        t_faq_ipv46_a: i18n::translate(
            locale,
            "Use <code>curl -4</code> or <code>curl -6</code> to specifically check your IPv4 or IPv6 address.",
        ),
        t_faq_json_q: i18n::translate(locale, "How do I get a JSON response?"),
        t_faq_json_a: i18n::translate(
            locale,
            "Send a request to the <code>/json</code> endpoint, or send a request with the <code>application/json</code> header.",
        ),
        t_faq_automate_q: i18n::translate(
            locale,
            "Can I call your service programmatically?",
        ),
        t_faq_automate_a1: i18n::translate(
            locale,
            "Sure, but please respect our rate limits. Normally it's 30 RPM, and under heavy load we may drop it to 10 or 5 RPM.",
        ),
        t_faq_automate_a2: i18n::translate(
            locale,
            "The API is meant for manual use or small projects. If your site uses our API to look up visitor IPs, please use a message queue so requests don't block. If your project has high traffic or needs low latency, you're better off using our open source offline database: <a href=\"https://github.com/NetworkCats/Merged-IP-Data\">Merged IP Database</a>. That's actually what this project uses under the hood.",
        ),
        t_faq_data_q: i18n::translate(locale, "Where does the IP data come from?"),
        t_faq_data_a: i18n::translate(
            locale,
            "Geolocation data mainly comes from the free databases by Maxmind and DB-IP. AS data comes from IPinfo's free database. Proxy data comes from my own <a href=\"https://github.com/NetworkCats/OpenProxyDB\">OpenProxyDB</a>.",
        ),
        t_faq_data_attr: i18n::translate(locale, "Data attributions:"),
        t_faq_selfhost_q: i18n::translate(locale, "Can I run my own instance?"),
        t_faq_selfhost_a: i18n::translate(
            locale,
            "Yes, the source code and database are both open source and on GitHub:",
        ),
        t_faq_diff_q: i18n::translate(
            locale,
            "How is this website different from other IP lookup sites?",
        ),
        t_faq_diff_a1: i18n::translate(
            locale,
            "The biggest thing is proxy detection. Our database covers proxy related data and the accuracy is really good. We've tested it privately against commercial databases like IPinfo's, and we hold up just as well, and actually do better on residential proxy detection.",
        ),
        t_faq_diff_a2: i18n::translate(
            locale,
            "We also have better IPv6 geolocation coverage since we merge multiple databases together, so the data ends up being more complete.",
        ),
        t_faq_diff_a3: i18n::translate(
            locale,
            "The UX is also better than most IP lookup sites out there. The site runs without any JavaScript, and we only pull in a minimal amount of external resources (fonts and icons), so it loads fast on the first visit.",
        ),
        t_faq_stable_q: i18n::translate(locale, "Is this service reliable?"),
        t_faq_stable_a1: i18n::translate(
            locale,
            "The whole thing is written in Rust, so performance is great. It can handle L7 DDoS traffic and stay up fine. It runs on a 2 core 2 GB VPS, which is already more than enough. The only thing that could cause real trouble is a large scale L7 DDoS, but we have Cloudflare in front of us.",
        ),
        t_faq_stable_a2: i18n::translate(
            locale,
            "It costs me $18.00/month to run, which is pretty cheap. I'm not going to shut it down over hosting costs anytime soon, and there won't be any ads.",
        ),
        t_faq_stable_a3: i18n::translate(
            locale,
            "This service is not going to rug pull on you.",
        ),
        t_faq_sponsor_q: i18n::translate(locale, "How can I support this service?"),
        t_faq_sponsor_a: i18n::translate(
            locale,
            "I accept crypto donations. If you want to contribute, reach out at:",
        ),
        t_footer_license: i18n::translate(locale, "Website source code licensed under"),
        t_footer_source: i18n::translate(locale, "Website Source"),
        t_footer_db: i18n::translate(locale, "IP Database"),
        t_footer_attribution: i18n::translate(locale, "IP data provided by:"),
        t_error: i18n::translate(locale, "Error"),
        t_go_home: i18n::translate(locale, "Go to Home"),
        t_table_aria: i18n::translate(locale, "IP address information"),
        t_faq_aria: i18n::translate(locale, "Frequently Asked Questions"),
        t_nav_aria: i18n::translate(locale, "Project links"),
        t_language: i18n::translate(locale, "Language"),
    }
}

struct Translations {
    t_title: &'static str,
    t_description: &'static str,
    t_search_label: &'static str,
    t_search_placeholder: &'static str,
    t_invalid_ip: &'static str,
    t_non_public_ip: &'static str,
    t_asn: &'static str,
    t_org: &'static str,
    t_country: &'static str,
    t_city: &'static str,
    t_proxy: &'static str,
    t_vpn: &'static str,
    t_hosting: &'static str,
    t_tor: &'static str,
    t_yes: &'static str,
    t_no: &'static str,
    t_faq: &'static str,
    t_faq_cli_q: &'static str,
    t_faq_cli_a: &'static str,
    t_faq_cli_query_current: &'static str,
    t_faq_cli_query_own: &'static str,
    t_faq_cli_json: &'static str,
    t_faq_cli_specific: &'static str,
    t_faq_ipv46_q: &'static str,
    t_faq_ipv46_a: &'static str,
    t_faq_json_q: &'static str,
    t_faq_json_a: &'static str,
    t_faq_automate_q: &'static str,
    t_faq_automate_a1: &'static str,
    t_faq_automate_a2: &'static str,
    t_faq_data_q: &'static str,
    t_faq_data_a: &'static str,
    t_faq_data_attr: &'static str,
    t_faq_selfhost_q: &'static str,
    t_faq_selfhost_a: &'static str,
    t_faq_diff_q: &'static str,
    t_faq_diff_a1: &'static str,
    t_faq_diff_a2: &'static str,
    t_faq_diff_a3: &'static str,
    t_faq_stable_q: &'static str,
    t_faq_stable_a1: &'static str,
    t_faq_stable_a2: &'static str,
    t_faq_stable_a3: &'static str,
    t_faq_sponsor_q: &'static str,
    t_faq_sponsor_a: &'static str,
    t_footer_license: &'static str,
    t_footer_source: &'static str,
    t_footer_db: &'static str,
    t_footer_attribution: &'static str,
    t_error: &'static str,
    t_go_home: &'static str,
    t_table_aria: &'static str,
    t_faq_aria: &'static str,
    t_nav_aria: &'static str,
    t_language: &'static str,
}

fn render_error_alert(state: &AppState, locale: Locale, query: &str, error: &AppError) -> Response {
    let t = build_translations(locale);
    let error_msg = i18n::translate(locale, error.message());
    let template = ErrorAlertTemplate {
        site_domain: state.site_domain.clone(),
        css_version: static_assets::asset_version("style.css"),
        query: query.to_owned(),
        error_message: error_msg.to_owned(),
        locale,
        lang_tag: locale.tag(),
        html_dir: locale.html_dir(),
        t_error: t.t_error,
        t_search_label: t.t_search_label,
        t_search_placeholder: t.t_search_placeholder,
        t_invalid_ip: t.t_invalid_ip,
        t_non_public_ip: t.t_non_public_ip,
        t_footer_license: t.t_footer_license,
        t_footer_source: t.t_footer_source,
        t_footer_db: t.t_footer_db,
        t_nav_aria: t.t_nav_aria,
        t_language: t.t_language,
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
    let t = build_translations(locale);
    let localized_message = i18n::translate(locale, message);
    let template = ErrorPageTemplate {
        site_domain: site_domain.clone(),
        css_version: static_assets::asset_version("style.css"),
        status_code: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("Error").to_owned(),
        error_message: localized_message.to_owned(),
        lang_tag: locale.tag(),
        html_dir: locale.html_dir(),
        t_go_home: t.t_go_home,
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
    let ua = headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // CLI clients and JSON requests bypass i18n entirely.
    if is_plain_text_agent(ua) && query.ip.is_none() {
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

    let locale = i18n::negotiate_locale(&headers);
    let mut redirect_path = format!("/{}", locale.tag());

    if let Some(ref ip) = query.ip {
        if !ip.trim().is_empty() {
            redirect_path.push_str("?ip=");
            redirect_path.push_str(&urlencoding::encode(ip));
        }
    }

    Redirect::temporary(&redirect_path).into_response()
}

pub async fn root_trailing_slash(
    Path(lang): Path<String>,
    Query(query): Query<IpQuery>,
) -> Response {
    let mut redirect_path = format!("/{lang}");
    if let Some(ref ip) = query.ip {
        if !ip.trim().is_empty() {
            redirect_path.push_str("?ip=");
            redirect_path.push_str(&urlencoding::encode(ip));
        }
    }
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

    let ua = headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if is_plain_text_agent(ua) && query.ip.is_none() {
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

    if let Some(ref ip_str) = query.ip {
        if ip_str.trim().is_empty() {
            return Redirect::temporary(&format!("/{}", locale.tag())).into_response();
        }
    }

    let query_str = query.ip.clone().unwrap_or_default();

    let ip = match resolve_ip(&headers, &query, state.dev_mode) {
        Ok(ip) => ip,
        Err(e) => return render_error_alert(&state, locale, &query_str, &e),
    };

    let info = match lookup_ip(&state.db, ip) {
        Ok(info) => info,
        Err(e) => return render_error_alert(&state, locale, &query_str, &e),
    };

    let t = build_translations(locale);

    let is_query = query.ip.is_some();
    let template = IndexTemplate {
        ip: info.ip.clone(),
        query: query.ip.unwrap_or_default(),
        is_query,
        site_domain: state.site_domain.clone(),
        css_version: static_assets::asset_version("style.css"),
        asn: format_asn(&info),
        asn_number: asn_number(&info),
        org: format_org(&info).to_owned(),
        country: format_country(&info, locale).to_owned(),
        city: format_city(&info, locale).to_owned(),
        is_proxy: info.proxy.is_proxy,
        is_vpn: info.proxy.is_vpn,
        is_hosting: info.proxy.is_hosting,
        is_tor: info.proxy.is_tor,
        locale,
        lang_tag: locale.tag(),
        html_dir: locale.html_dir(),
        t_title: t.t_title,
        t_description: t.t_description,
        t_search_label: t.t_search_label,
        t_search_placeholder: t.t_search_placeholder,
        t_invalid_ip: t.t_invalid_ip,
        t_non_public_ip: t.t_non_public_ip,
        t_asn: t.t_asn,
        t_org: t.t_org,
        t_country: t.t_country,
        t_city: t.t_city,
        t_proxy: t.t_proxy,
        t_vpn: t.t_vpn,
        t_hosting: t.t_hosting,
        t_tor: t.t_tor,
        t_yes: t.t_yes,
        t_no: t.t_no,
        t_faq: t.t_faq,
        t_faq_cli_q: t.t_faq_cli_q,
        t_faq_cli_a: t.t_faq_cli_a,
        t_faq_cli_query_current: t.t_faq_cli_query_current,
        t_faq_cli_query_own: t.t_faq_cli_query_own,
        t_faq_cli_json: t.t_faq_cli_json,
        t_faq_cli_specific: t.t_faq_cli_specific,
        t_faq_ipv46_q: t.t_faq_ipv46_q,
        t_faq_ipv46_a: t.t_faq_ipv46_a,
        t_faq_json_q: t.t_faq_json_q,
        t_faq_json_a: t.t_faq_json_a,
        t_faq_automate_q: t.t_faq_automate_q,
        t_faq_automate_a1: t.t_faq_automate_a1,
        t_faq_automate_a2: t.t_faq_automate_a2,
        t_faq_data_q: t.t_faq_data_q,
        t_faq_data_a: t.t_faq_data_a,
        t_faq_data_attr: t.t_faq_data_attr,
        t_faq_selfhost_q: t.t_faq_selfhost_q,
        t_faq_selfhost_a: t.t_faq_selfhost_a,
        t_faq_diff_q: t.t_faq_diff_q,
        t_faq_diff_a1: t.t_faq_diff_a1,
        t_faq_diff_a2: t.t_faq_diff_a2,
        t_faq_diff_a3: t.t_faq_diff_a3,
        t_faq_stable_q: t.t_faq_stable_q,
        t_faq_stable_a1: t.t_faq_stable_a1,
        t_faq_stable_a2: t.t_faq_stable_a2,
        t_faq_stable_a3: t.t_faq_stable_a3,
        t_faq_sponsor_q: t.t_faq_sponsor_q,
        t_faq_sponsor_a: t.t_faq_sponsor_a,
        t_footer_license: t.t_footer_license,
        t_footer_source: t.t_footer_source,
        t_footer_db: t.t_footer_db,
        t_footer_attribution: t.t_footer_attribution,
        t_table_aria: t.t_table_aria,
        t_faq_aria: t.t_faq_aria,
        t_nav_aria: t.t_nav_aria,
        t_language: t.t_language,
        all_locales: Locale::ALL,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => render_error_alert(&state, locale, &query_str, &AppError::TemplateRenderFailed),
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
    let mut disallow = String::new();
    for path in &[
        "/json",
        "/ip",
        "/asn",
        "/org",
        "/country",
        "/city",
        "/proxy",
        "/vpn",
        "/hosting",
        "/tor",
        "/health",
        "/openapi.json",
    ] {
        disallow.push_str(&format!("Disallow: {path}\n"));
    }

    let body = format!(
        "User-agent: *\n\
         Allow: /\n\
         {disallow}\
         \n\
         Sitemap: https://{}/sitemap.xml\n",
        state.site_domain
    );
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body).into_response()
}

pub async fn sitemap_xml(State(state): State<AppState>) -> Response {
    let mut urls = String::new();
    let domain = &state.site_domain;

    for locale in Locale::ALL {
        let tag = locale.tag();
        urls.push_str(&format!("  <url>\n    <loc>https://{domain}/{tag}</loc>\n"));
        for alt in Locale::ALL {
            let alt_tag = alt.tag();
            urls.push_str(&format!(
                "    <xhtml:link rel=\"alternate\" hreflang=\"{alt_tag}\" href=\"https://{domain}/{alt_tag}\"/>\n"
            ));
        }
        urls.push_str(
            &format!("    <xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"https://{domain}/\"/>\n"),
        );
        urls.push_str("  </url>\n");
    }

    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"\n\
         \x20       xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n\
         {urls}\
         </urlset>\n"
    );
    (
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        body,
    )
        .into_response()
}

pub async fn favicon() -> Response {
    Redirect::permanent("/static/icons/favicon.ico").into_response()
}

pub async fn manifest(State(state): State<AppState>) -> Response {
    let manifest = serde_json::json!({
        "name": state.site_domain.as_ref(),
        "short_name": state.site_domain.as_ref(),
        "start_url": "/",
        "display": "standalone",
        "background_color": "#FFFFFF",
        "theme_color": "#007AFF",
        "icons": [
            {
                "src": "/static/icons/android-chrome-192x192.png",
                "sizes": "192x192",
                "type": "image/png"
            },
            {
                "src": "/static/icons/android-chrome-512x512.png",
                "sizes": "512x512",
                "type": "image/png"
            },
            {
                "src": "/static/icons/android-chrome-512x512.png",
                "sizes": "512x512",
                "type": "image/png",
                "purpose": "maskable"
            }
        ]
    });

    let body = serde_json::to_string_pretty(&manifest).expect("manifest serialization cannot fail");
    (
        [(header::CONTENT_TYPE, "application/manifest+json; charset=utf-8")],
        body,
    )
        .into_response()
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
