use crate::billing::StripePrices;
use crate::ratelimit::Limiter;
use ed25519_dalek::SigningKey;
use sqlx::PgPool;
use std::sync::Arc;

/// Stripe-related boot inputs, grouped so `AppState::new` stays within a sane
/// argument count. All fields are independently optional — Stripe is wholly
/// optional in dev.
#[derive(Default)]
pub struct StripeConfig {
    pub client: Option<stripe::Client>,
    pub prices: StripePrices,
    pub webhook_secret: Option<String>,
}

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
    /// Stripe Price ID ↔ plan map (from `STRIPE_PRICE_*`). Empty when unset.
    pub stripe_prices: StripePrices,
    /// `STRIPE_WEBHOOK_SECRET`. `None` if unset, in which case the webhook
    /// endpoint rejects everything (it can't verify signatures).
    pub stripe_webhook_secret: Option<Arc<String>>,
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
        stripe: StripeConfig,
        cookie_secure: bool,
    ) -> Self {
        Self {
            server_keypair: Arc::new(server_keypair),
            bootstrap_key: bootstrap_key.map(Arc::new),
            rate_limiter,
            db,
            stripe: stripe.client.map(Arc::new),
            stripe_prices: stripe.prices,
            stripe_webhook_secret: stripe.webhook_secret.map(Arc::new),
            cookie_secure,
        }
    }
}
