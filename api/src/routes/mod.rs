use crate::auth as middleware_auth;
use crate::ratelimit;
use crate::state::AppState;
use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};

pub mod admin;
pub mod auth;
pub mod dashboard;
pub mod proofs;
pub mod usage;

/// Dashboard endpoints (`/dashboard/*`). Session-cookie auth via
/// `require_user_session`. No rate limit: the current limiter keys on the
/// `X-API-Key` header (or the literal "dev" if missing), so cookie-authed
/// traffic would all collapse into the same bucket and block every
/// dashboard user once any one of them hit the threshold. Per-session or
/// per-IP limiting can come back in a later phase.
pub fn dashboard_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/dashboard/proofs", get(dashboard::list_proofs))
        .route("/dashboard/usage", get(dashboard::get_usage))
        .route(
            "/dashboard/keys",
            post(dashboard::create_key).get(dashboard::list_keys),
        )
        .route("/dashboard/keys/:id", delete(dashboard::delete_key))
        .route_layer(middleware::from_fn_with_state(
            state,
            middleware_auth::require_user_session,
        ))
}

/// Auth endpoints (`/auth/*`). Public — no middleware. Register/login
/// issue session cookies; logout deletes them. Rate-limited to deter
/// credential stuffing.
pub fn auth_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route_layer(middleware::from_fn_with_state(
            state,
            ratelimit::rate_limit_middleware,
        ))
}

pub fn v1_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/proofs", post(proofs::submit).get(proofs::list))
        .route("/v1/proofs/:proof_id", get(proofs::get_proof))
        .route("/v1/usage", get(usage::get_usage))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_auth::require_customer_key,
        ))
        .route_layer(middleware::from_fn_with_state(
            state,
            ratelimit::rate_limit_middleware,
        ))
}

/// Admin endpoints (`/admin/*`). Bootstrap-key auth only — no rate limit,
/// since these are internal/infrequent operations.
pub fn admin_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/admin/customers",
            post(admin::create_customer).get(admin::list_customers),
        )
        .route("/admin/customers/:id", delete(admin::delete_customer))
        .route_layer(middleware::from_fn_with_state(
            state,
            middleware_auth::require_bootstrap_key,
        ))
}
