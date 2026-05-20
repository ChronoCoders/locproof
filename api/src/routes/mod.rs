use crate::auth;
use crate::ratelimit;
use crate::state::AppState;
use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};

pub mod admin;
pub mod proofs;

pub fn v1_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/proofs", post(proofs::submit))
        .route("/v1/proofs/:proof_id", get(proofs::get_proof))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_bootstrap_key,
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
            auth::require_bootstrap_key,
        ))
}
