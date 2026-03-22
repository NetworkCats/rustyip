use std::sync::Arc;

use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Redirect, Response};

use crate::i18n::Locale;

use super::AppState;

pub async fn openapi_json(State(state): State<AppState>) -> Response {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        String::from(state.openapi_json.as_ref()),
    )
        .into_response()
}

pub fn build_openapi_json(site_domain: &str) -> Arc<str> {
    let mut spec: serde_json::Value =
        serde_json::from_str(include_str!("../../openapi.json")).expect("openapi.json must be valid");

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
        [(
            header::CONTENT_TYPE,
            "application/manifest+json; charset=utf-8",
        )],
        body,
    )
        .into_response()
}
