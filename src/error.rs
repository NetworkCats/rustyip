use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub enum AppError {
    IpNotFound,
    InvalidIp,
    NonPublicIp,
    MissingClientIp,
    DbLookupFailed,
    TemplateRenderFailed,
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::IpNotFound => StatusCode::NOT_FOUND,
            Self::InvalidIp | Self::NonPublicIp => StatusCode::BAD_REQUEST,
            Self::MissingClientIp => StatusCode::BAD_REQUEST,
            Self::DbLookupFailed => StatusCode::INTERNAL_SERVER_ERROR,
            Self::TemplateRenderFailed => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::IpNotFound => "IP not found in database",
            Self::InvalidIp => "Invalid IP address",
            Self::NonPublicIp => "Only public IP addresses can be queried",
            Self::MissingClientIp => "Missing client IP address",
            Self::DbLookupFailed => "Database lookup failed",
            Self::TemplateRenderFailed => "Template render failed",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status_code(), self.message()).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    async fn check_error_response(
        error: AppError,
        expected_status: StatusCode,
        expected_body: &str,
    ) {
        let response = error.into_response();
        assert_eq!(response.status(), expected_status);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert_eq!(text, expected_body);
    }

    #[tokio::test]
    async fn ip_not_found_returns_404() {
        check_error_response(
            AppError::IpNotFound,
            StatusCode::NOT_FOUND,
            "IP not found in database",
        )
        .await;
    }

    #[tokio::test]
    async fn invalid_ip_returns_400() {
        check_error_response(
            AppError::InvalidIp,
            StatusCode::BAD_REQUEST,
            "Invalid IP address",
        )
        .await;
    }

    #[tokio::test]
    async fn non_public_ip_returns_400() {
        check_error_response(
            AppError::NonPublicIp,
            StatusCode::BAD_REQUEST,
            "Only public IP addresses can be queried",
        )
        .await;
    }

    #[tokio::test]
    async fn missing_client_ip_returns_400() {
        check_error_response(
            AppError::MissingClientIp,
            StatusCode::BAD_REQUEST,
            "Missing client IP address",
        )
        .await;
    }

    #[tokio::test]
    async fn db_lookup_failed_returns_500() {
        check_error_response(
            AppError::DbLookupFailed,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database lookup failed",
        )
        .await;
    }

    #[tokio::test]
    async fn template_render_failed_returns_500() {
        check_error_response(
            AppError::TemplateRenderFailed,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Template render failed",
        )
        .await;
    }
}
