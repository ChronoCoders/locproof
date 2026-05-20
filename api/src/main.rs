#![deny(warnings)]

mod auth;
mod db;
mod error;
mod keystore;
mod ratelimit;
mod routes;
mod state;

use axum::{extract::State, http::StatusCode, routing::get, Router};
use base64::{engine::general_purpose::STANDARD, Engine};
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let (keypair, origin) = keystore::load_or_generate_keypair()?;
    match origin {
        keystore::KeyOrigin::Loaded => println!("Loaded existing server keypair"),
        keystore::KeyOrigin::Generated => println!("Generated new server keypair"),
    }
    let pubkey_b64 = STANDARD.encode(keypair.verifying_key().as_bytes());
    println!("Server public key: {pubkey_b64}");

    let bootstrap_key = match std::env::var("LOCPROOF_API_KEY") {
        Ok(k) => {
            println!("Bootstrap admin key loaded");
            Some(k)
        }
        Err(_) => {
            if std::env::var("LOCPROOF_DEV").as_deref() != Ok("1") {
                anyhow::bail!(
                    "LOCPROOF_API_KEY not set. Set it for production, or set LOCPROOF_DEV=1 to run without auth."
                );
            }
            eprintln!(
                "WARNING: LOCPROOF_API_KEY not set — running in dev mode with no admin auth"
            );
            None
        }
    };

    let rate_per_min: u32 = std::env::var("LOCPROOF_RATE_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    let rate_limiter = ratelimit::create_limiter(rate_per_min)?;
    println!("Rate limit: {rate_per_min} req/min per API key");

    let pool = db::create_pool().await?;
    db::run_migrations(&pool).await?;
    println!("Database connected, migrations applied");

    let state = AppState::new(keypair, bootstrap_key, rate_limiter, pool);

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .merge(routes::v1_router(state.clone()))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("locproof-api listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
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
