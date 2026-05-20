use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

/// Middleware that requires `X-API-Key` to match the configured server key.
///
/// If `AppState::api_key` is `None` (no `LOCPROOF_API_KEY` env var at boot),
/// the middleware allows every request through and the server runs in
/// "dev mode" with no authentication.
pub async fn require_api_key(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let Some(expected) = state.api_key.as_ref() else {
        return Ok(next.run(request).await);
    };

    let provided = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    match provided {
        Some(key) if constant_time_eq(key.as_bytes(), expected.as_bytes()) => {
            Ok(next.run(request).await)
        }
        _ => Err(ApiError::Unauthorized),
    }
}

/// Constant-time byte-slice comparison. Returns `false` on length mismatch
/// (length is not a secret); otherwise compares every byte regardless of
/// where the first difference is.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_strings_match() {
        assert!(constant_time_eq(b"lp_live_abc", b"lp_live_abc"));
    }

    #[test]
    fn different_strings_dont_match() {
        assert!(!constant_time_eq(b"lp_live_abc", b"lp_live_xyz"));
    }

    #[test]
    fn different_lengths_dont_match() {
        assert!(!constant_time_eq(b"short", b"much-longer-key"));
    }

    #[test]
    fn empty_strings_match() {
        assert!(constant_time_eq(b"", b""));
    }
}
