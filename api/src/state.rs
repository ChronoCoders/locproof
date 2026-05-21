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
    /// `None` if `STRIPE_SECRET_KEY` was unset at boot; `/auth/register`
    /// then skips the Stripe customer create and leaves
    /// `customers.stripe_customer_id` NULL.
    pub stripe: Option<Arc<stripe::Client>>,
    /// Whether to set the `Secure` attribute on session cookies. True in
    /// production (HTTPS), false in `LOCPROOF_DEV=1` so localhost http
    /// requests still receive the cookie.
    pub cookie_secure: bool,
}

impl AppState {
    pub fn new(
        server_keypair: SigningKey,
        bootstrap_key: Option<String>,
        rate_limiter: Arc<Limiter>,
        db: PgPool,
        stripe: Option<stripe::Client>,
        cookie_secure: bool,
    ) -> Self {
        Self {
            server_keypair: Arc::new(server_keypair),
            bootstrap_key: bootstrap_key.map(Arc::new),
            rate_limiter,
            db,
            stripe: stripe.map(Arc::new),
            cookie_secure,
        }
    }
}
