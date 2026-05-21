-- Phase 4a C1. Splits the single `customers.api_key_hash` column into a
-- dedicated `api_keys` table (one customer can hold many keys), and adds
-- the `plan` + `stripe_customer_id` columns that 4c billing will read.

CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    customer_id UUID NOT NULL REFERENCES customers(id),
    key_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL DEFAULT 'default',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);
CREATE INDEX idx_api_keys_customer ON api_keys(customer_id);

-- Backfill: every existing customer's single key becomes a row in api_keys.
INSERT INTO api_keys (customer_id, key_hash, name)
    SELECT id, api_key_hash, 'default' FROM customers;

ALTER TABLE customers DROP COLUMN api_key_hash;

ALTER TABLE customers
    ADD COLUMN plan TEXT NOT NULL DEFAULT 'free';
ALTER TABLE customers
    ADD COLUMN stripe_customer_id TEXT UNIQUE;
