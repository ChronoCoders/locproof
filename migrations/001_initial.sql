-- LocProof initial schema. Tables: customers, usage, proofs.

CREATE TABLE customers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    api_key_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    customer_id UUID NOT NULL REFERENCES customers(id),
    month DATE NOT NULL,
    proof_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE(customer_id, month)
);

CREATE TABLE proofs (
    id UUID PRIMARY KEY,
    customer_id UUID NOT NULL REFERENCES customers(id),
    device_a_pubkey BYTEA NOT NULL,
    device_b_pubkey BYTEA NOT NULL,
    proximity_score DOUBLE PRECISION NOT NULL,
    server_signature BYTEA NOT NULL,
    proof_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_proofs_customer ON proofs(customer_id);
CREATE INDEX idx_proofs_created ON proofs(created_at);
CREATE INDEX idx_usage_customer_month ON usage(customer_id, month);
