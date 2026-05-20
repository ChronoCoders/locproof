use crate::ratelimit::Limiter;
use ed25519_dalek::SigningKey;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub server_keypair: Arc<SigningKey>,
    /// Bootstrap admin key from `LOCPROOF_API_KEY`. Gates `/admin/*` only.
    /// `/v1/*` uses per-customer keys looked up via [`AppState::db`].
    /// `None` disables admin auth (dev mode); customer auth on `/v1/*` is
    /// always enforced, so dev users should mint a key via the (then
    /// unauthenticated) `POST /admin/customers`.
    pub bootstrap_key: Option<Arc<String>>,
    pub rate_limiter: Arc<Limiter>,
    pub db: PgPool,
}

impl AppState {
    pub fn new(
        server_keypair: SigningKey,
        bootstrap_key: Option<String>,
        rate_limiter: Arc<Limiter>,
        db: PgPool,
    ) -> Self {
        Self {
            server_keypair: Arc::new(server_keypair),
            bootstrap_key: bootstrap_key.map(Arc::new),
            rate_limiter,
            db,
        }
    }
}
