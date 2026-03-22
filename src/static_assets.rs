//! Embedded static file serving with immutable cache headers.

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::http::header::{self, HeaderValue};
use axum::response::{IntoResponse, Response};

use static_files::Resource;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

static ASSETS: LazyLock<HashMap<&'static str, Resource>> = LazyLock::new(generate);

pub fn asset_version(filename: &str) -> u64 {
    ASSETS.get(filename).map(|r| r.modified).unwrap_or(0)
}

pub async fn serve_static(Path(path): Path<String>) -> Response {
    let Some(resource) = ASSETS.get(path.as_str()) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static(resource.mime_type),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            ),
        ],
        resource.data,
    )
        .into_response()
}
