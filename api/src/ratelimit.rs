use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use governor::{
    clock::{Clock, DefaultClock},
    state::keyed::DashMapStateStore,
    Quota, RateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Token-bucket rate limiter keyed by API key string.
pub type Limiter = RateLimiter<String, DashMapStateStore<String>, DefaultClock>;

/// Build a per-key rate limiter at `requests_per_minute` requests per bucket.
///
/// Returns an error if `requests_per_minute` is zero — a zero limit would
/// reject every request and is almost certainly a misconfiguration.
pub fn create_limiter(requests_per_minute: u32) -> anyhow::Result<Arc<Limiter>> {
    let rate = NonZeroU32::new(requests_per_minute)
        .ok_or_else(|| anyhow::anyhow!("LOCPROOF_RATE_LIMIT must be > 0"))?;
    let quota = Quota::per_minute(rate);
    Ok(Arc::new(RateLimiter::keyed(quota)))
}

/// Middleware: enforce the rate limit, keyed by the `X-API-Key` header (or
/// the literal "dev" if no key is present, which only happens in dev mode).
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| "dev".to_string());

    match state.rate_limiter.check_key(&key) {
        Ok(_) => Ok(next.run(request).await),
        Err(not_until) => {
            let wait = not_until.wait_time_from(DefaultClock::default().now());
            Err(ApiError::RateLimited {
                retry_after_secs: wait.as_secs().max(1),
            })
        }
    }
}
