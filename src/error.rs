use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub enum AppError {
    IpNotFound,
    InvalidIp,
    DbLookupFailed,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::IpNotFound => (StatusCode::NOT_FOUND, "IP not found in database"),
            Self::InvalidIp => (StatusCode::BAD_REQUEST, "Invalid IP address"),
            Self::DbLookupFailed => (StatusCode::INTERNAL_SERVER_ERROR, "Database lookup failed"),
        };
        (status, message).into_response()
    }
}
