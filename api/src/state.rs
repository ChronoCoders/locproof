use crate::ratelimit::Limiter;
use ed25519_dalek::SigningKey;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub server_keypair: Arc<SigningKey>,
    /// Expected `X-API-Key` value. `None` disables auth (dev mode).
    /// Phase 2 will turn this into a bootstrap-only admin key alongside
    /// per-customer keys looked up via [`AppState::db`].
    pub api_key: Option<Arc<String>>,
    pub rate_limiter: Arc<Limiter>,
    pub db: PgPool,
}

impl AppState {
    pub fn new(
        server_keypair: SigningKey,
        api_key: Option<String>,
        rate_limiter: Arc<Limiter>,
        db: PgPool,
    ) -> Self {
        Self {
            server_keypair: Arc::new(server_keypair),
            api_key: api_key.map(Arc::new),
            rate_limiter,
            db,
        }
    }
}
