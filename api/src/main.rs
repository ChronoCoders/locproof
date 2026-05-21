#![deny(warnings)]

use base64::{engine::general_purpose::STANDARD, Engine};
use locproof_api::{build_app, db, keystore, ratelimit, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let (keypair, origin) = keystore::load_or_generate_keypair()?;
    match origin {
        keystore::KeyOrigin::Loaded => tracing::info!("Loaded existing server keypair"),
        keystore::KeyOrigin::Generated => tracing::info!("Generated new server keypair"),
    }
    let pubkey_b64 = STANDARD.encode(keypair.verifying_key().as_bytes());
    tracing::info!("Server public key: {pubkey_b64}");

    let bootstrap_key = match std::env::var("LOCPROOF_API_KEY") {
        Ok(k) => {
            if !is_well_formed_bootstrap_key(&k) {
                tracing::warn!(
                    "LOCPROOF_API_KEY does not match the expected format \
                     `lp_admin_<32 hex>`. The server will use it as-is."
                );
            }
            tracing::info!("Bootstrap admin key loaded");
            Some(k)
        }
        Err(_) => {
            if std::env::var("LOCPROOF_DEV").as_deref() != Ok("1") {
                anyhow::bail!(
                    "LOCPROOF_API_KEY not set. Set it for production, or set LOCPROOF_DEV=1 to run without auth."
                );
            }
            tracing::warn!("LOCPROOF_API_KEY not set — running in dev mode with no admin auth");
            None
        }
    };

    let rate_per_min: u32 = std::env::var("LOCPROOF_RATE_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    let rate_limiter = ratelimit::create_limiter(rate_per_min)?;
    tracing::info!("Rate limit: {rate_per_min} req/min per API key");

    let pool = db::create_pool().await?;
    db::run_migrations(&pool).await?;
    tracing::info!("Database connected, migrations applied");

    let state = AppState::new(keypair, bootstrap_key, rate_limiter, pool);
    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("locproof-api listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

/// Convention: bootstrap admin key is `lp_admin_` followed by 32 lowercase
/// hex characters (16 bytes of randomness). Not enforced — operators may
/// supply any value — but startup warns if the convention is violated so
/// a misconfigured key (e.g. an `lp_live_…` customer key pasted in by
/// mistake) is visible in the logs.
fn is_well_formed_bootstrap_key(key: &str) -> bool {
    let Some(rest) = key.strip_prefix("lp_admin_") else {
        return false;
    };
    rest.len() == 32 && rest.bytes().all(|b| b.is_ascii_hexdigit())
}
