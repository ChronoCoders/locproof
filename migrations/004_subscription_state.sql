-- Subscription linkage synced from Stripe (Phase 4c). plan already exists
-- (migration 002); these track the active subscription behind a paid plan so
-- the webhook can reconcile plan changes and cancellations.
ALTER TABLE customers
    ADD COLUMN stripe_subscription_id TEXT UNIQUE;
ALTER TABLE customers
    ADD COLUMN subscription_status TEXT;
