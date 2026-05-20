use crate::auth;
use crate::ratelimit;
use crate::state::AppState;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};

pub mod proofs;

pub fn v1_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/proofs", post(proofs::submit))
        .route("/v1/proofs/:proof_id", get(proofs::get_proof))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        ))
        .route_layer(middleware::from_fn_with_state(
            state,
            ratelimit::rate_limit_middleware,
        ))
}
