use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// Map any error into `ApiError::Internal`, logging the underlying cause
/// first. Use at every `.map_err(...)` that collapses to a 500 — DB,
/// crypto, serde — so 500s aren't silent.
pub fn internal_err<E: std::fmt::Debug>(e: E) -> ApiError {
    tracing::error!("internal error: {e:?}");
    ApiError::Internal
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("invalid request: {0}")]
    BadRequest(String),
    #[error("invalid signature for device {0}")]
    InvalidSignature(&'static str),
    #[error("missing or invalid API key")]
    Unauthorized,
    #[error("device timestamps differ by {delta_seconds}s (max allowed: {max})")]
    TimestampSkew { delta_seconds: i64, max: i64 },
    #[error("rate limit exceeded; retry in {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("internal server error")]
    Internal,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::BadRequest(_) | ApiError::TimestampSkew { .. } => StatusCode::BAD_REQUEST,
            ApiError::InvalidSignature(_) | ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let retry_after = match &self {
            ApiError::RateLimited { retry_after_secs } => Some(*retry_after_secs),
            _ => None,
        };
        let body = Json(ErrorResponse {
            error: self.to_string(),
        });
        let mut response = (status, body).into_response();
        if let Some(secs) = retry_after {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(secs));
            response
                .headers_mut()
                .insert("X-RateLimit-Reset", HeaderValue::from(secs));
            response
                .headers_mut()
                .insert("X-RateLimit-Remaining", HeaderValue::from_static("0"));
        }
        response
    }
}
