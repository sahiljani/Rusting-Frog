use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub struct ApiError {
    status: u16,
    error_type: &'static str,
    detail: String,
}

impl ApiError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self { status: 404, error_type: "not_found", detail: msg.into() }
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self { status: 401, error_type: "unauthorized", detail: msg.into() }
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self { status: 422, error_type: "validation_error", detail: msg.into() }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self { status: 500, error_type: "internal_error", detail: msg.into() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let body = json!({
            "type": self.error_type,
            "title": status.canonical_reason().unwrap_or("Error"),
            "status": self.status,
            "detail": self.detail,
        });

        (
            status,
            [("content-type", "application/problem+json")],
            body.to_string(),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!(error = %e, "database error");
        Self::internal("database error")
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        Self::internal(e.to_string())
    }
}
