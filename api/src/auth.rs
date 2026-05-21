use crate::error::{internal_err, ApiError};
use crate::models::{api_key, session};
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;

/// Middleware that requires `X-API-Key` to match the bootstrap admin key
/// (`LOCPROOF_API_KEY`).
///
/// If `AppState::bootstrap_key` is `None` (env var unset at boot), the
/// middleware allows every request through and the server runs in dev mode
/// with no admin authentication.
pub async fn require_bootstrap_key(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let Some(expected) = state.bootstrap_key.as_ref() else {
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

/// Middleware that requires `X-API-Key` to be the plaintext of an active
/// customer key. The provided value is SHA-256 hashed and joined against
/// `api_keys` → `customers`, requiring both rows to be active.
///
/// No dev-mode bypass: customer keys are DB-backed by design. In dev mode
/// (`LOCPROOF_DEV=1`, no `LOCPROOF_API_KEY`), mint one via the unauthenticated
/// `POST /admin/customers` and use the returned key on `/v1/*`.
pub async fn require_customer_key(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let provided = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    let hash = api_key::hash_api_key(provided);

    let customer_id = sqlx::query_scalar!(
        r#"
        SELECT c.id
        FROM api_keys k
        JOIN customers c ON c.id = k.customer_id
        WHERE k.key_hash = $1 AND k.is_active = true AND c.is_active = true
        "#,
        hash,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(internal_err)?
    .ok_or(ApiError::Unauthorized)?;

    request.extensions_mut().insert(customer_id);
    Ok(next.run(request).await)
}

/// Name of the session cookie that `/auth/*` sets and the dashboard
/// middleware reads. Sent only with same-site requests (SameSite=Strict).
pub const SESSION_COOKIE: &str = "lp_session";

/// Middleware that requires a valid dashboard session cookie. Looks up the
/// token in `sessions`, slides the expiry, and injects the session's
/// `user_id` and `customer_id` into request extensions for handlers.
///
/// No dev-mode bypass: the session table is authoritative.
pub async fn require_user_session(
    State(state): State<AppState>,
    jar: CookieJar,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let token = jar
        .get(SESSION_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or(ApiError::Unauthorized)?;

    let active = session::find_active_and_touch(&state.db, &token)
        .await
        .map_err(internal_err)?
        .ok_or(ApiError::Unauthorized)?;

    request.extensions_mut().insert(active.user_id);
    request
        .extensions_mut()
        .insert(CustomerId(active.customer_id));
    Ok(next.run(request).await)
}

/// Newtype wrapper for the session-derived `customer_id`. Distinct from the
/// bare `Uuid` injected by `require_customer_key` so handlers don't
/// silently extract the wrong source.
#[derive(Clone, Copy)]
pub struct CustomerId(pub uuid::Uuid);

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
