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
pub mod proofs;
pub mod usage;

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
