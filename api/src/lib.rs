#![deny(warnings)]

pub mod auth;
pub mod db;
pub mod error;
pub mod keystore;
pub mod models;
pub mod ratelimit;
pub mod routes;
pub mod state;

use axum::{extract::State, http::StatusCode, routing::get, Router};
use state::AppState;

/// Assemble the full application router. Public so integration tests can
/// drive the same router the binary serves.
pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .merge(routes::v1_router(state.clone()))
        .merge(routes::admin_router(state.clone()))
        .merge(routes::auth_router(state.clone()))
        .with_state(state)
}

async fn root() -> &'static str {
    "locproof-api"
}

/// Liveness + readiness check. Pings the database; returns 503 if the pool
/// can't satisfy a trivial query.
async fn health(State(state): State<AppState>) -> Result<&'static str, StatusCode> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .map(|_| "ok")
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}
