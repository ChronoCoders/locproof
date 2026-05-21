# Changelog

All notable changes to LocProof are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.2.0] — 2026-05-20

Phase 2 — Auth refactor and customer management.

### Added

- **Bootstrap admin key.** `LOCPROOF_API_KEY` is now the bootstrap admin key
  (convention: `lp_admin_<32 hex>`) and gates `/admin/*` only. A startup
  warning fires if the value doesn't match the conventional format; operators
  may supply any string.
- **Per-customer keys.** `/v1/*` now requires a customer key
  (`lp_live_<32 hex>`) issued via `POST /admin/customers`. Plaintext is
  returned exactly once at creation; only the SHA-256 hash is persisted.
- **Customer model.** `api/src/models/customer.rs` with `generate_api_key`,
  `hash_api_key`, and `create` / `list` / `deactivate` helpers using
  compile-time-checked `sqlx::query!` macros.
- **`/admin/customers` endpoints.**
  - `POST /admin/customers` — create a customer; returns id, name, plaintext
    `api_key`, and `created_at`.
  - `GET /admin/customers` — list customers (no key/hash returned).
  - `DELETE /admin/customers/:id` — soft delete (`is_active = false`).
    Idempotent: 200 regardless of whether the row was already inactive or
    didn't exist.
- **`require_customer_key` middleware.** SHA-256 hashes the `X-API-Key`
  header and looks it up against `customers.api_key_hash` with
  `is_active = true`. No dev-mode bypass — in dev, mint a key via the
  (then unauthenticated) `POST /admin/customers`.
- **sqlx offline cache.** `.sqlx/` checked in so the workspace builds
  without a live database (run `cargo sqlx prepare --workspace` after
  adding queries).

### Changed

- `AppState::api_key` → `AppState::bootstrap_key`; `auth::require_api_key`
  → `auth::require_bootstrap_key`.
- `admin_router` is not rate-limited — admin operations are internal and
  infrequent.
- README quickstart rewritten to walk through bootstrap key → mint customer
  → use customer key on `/v1/*`.

### Security notes

- Customer-key lookup is via SQL equality on the SHA-256 hash, not constant
  time. The plaintext has 128 bits of entropy and the hash is indexed and not
  secret; standard practice for hashed-key tables.
- Bootstrap key format is conventional only — startup warns but does not
  reject malformed values.

### Not yet implemented

- Proof storage and `GET /v1/proofs/:id` retrieval (Phase 3).
- Usage counting and monthly aggregation (Phase 3).

## [v0.1.0] — 2026-05-20

Phase 1 — API foundation and PostgreSQL storage.

### Added

- **`core/`** — pure Rust protocol primitives, no I/O.
  - `DeviceAttestation` (Ed25519-signed signal snapshot per device).
  - `ProximityProof` with `id`, timestamps, two attestations, proximity score,
    and an outer server signature binding the score.
  - `SignalSnapshot` carrying GPS, BLE, and barometer readings plus the
    device's BLE-advertising id for mutual-visibility detection.
  - `verify::verify_proof` — checks both device signatures, the
    inter-device timestamp window, and the server signature.
  - `verify::sign_proof` — server-side signing using bincode as the
    canonical encoding (replaces serde_json, which was non-canonical).
  - `scoring::calculate_proximity_score` — weighted blend of BLE (0.5),
    barometer (0.3), and GPS (0.2). Weights renormalised over only the
    signal classes that produced a value.

- **`api/`** — Axum REST API.
  - `POST /v1/proofs` — submit two device attestations; the server verifies
    them, computes the score, signs the proof, and returns
    `{ proof_id, proximity_score, verified, timestamp }`.
  - `GET /v1/proofs/:proof_id` — endpoint wired; returns `501 Not Implemented`
    pending persistence (lands in Phase 3).
  - `GET /health` — readiness check; pings PostgreSQL with `SELECT 1` and
    returns `503` if the pool can't satisfy the query.
  - Base64-encoded wire format at the API boundary; core stays binary.

- **Auth.** `X-API-Key` header, constant-time comparison, JSON error on 401.
  `LOCPROOF_API_KEY` env var configures the key; `LOCPROOF_DEV=1` is required
  to start without one (otherwise startup fails with a clear message).

- **Rate limiting.** Token bucket (`governor` crate), keyed per API key,
  default 100 req/min (override via `LOCPROOF_RATE_LIMIT`). 429 responses
  carry `Retry-After`, `X-RateLimit-Reset`, and `X-RateLimit-Remaining`.

- **Keystore.** Server Ed25519 signing key persists at `data/server.key`
  (mode `0600`, gitignored). Same key across restarts — previously issued
  proofs remain verifiable.

- **PostgreSQL storage.** Connection pool via `sqlx 0.7`, migrations embedded
  at compile time. Initial schema: `customers`, `usage`, `proofs` (with the
  full signed proof persisted as JSONB) plus indexes for per-customer and
  time-range queries. `/health` doubles as a DB readiness check.

- **Project docs.** Protocol spec, mobile-signal availability research,
  threat model.

### Security notes

- The server signing key is stored in plaintext on disk with `0600`. Adequate
  for a single-tenant pilot; HSM/KMS-backed signing is the right answer
  before customer rollout.
- No key rotation path yet — losing `data/server.key` invalidates every
  previously issued proof.
- API key in v0.1.0 is a single shared secret. Per-customer keys with hashed
  storage land in Phase 2.

### Not yet implemented

- Per-customer API keys backed by `customers` table (Phase 2).
- Admin endpoints for customer create/list (Phase 2).
- Proof storage and `GET /v1/proofs/:id` retrieval (Phase 3).
- Usage counting and monthly aggregation (Phase 3).

[v0.2.0]: https://github.com/ChronoCoders/locproof/releases/tag/v0.2.0
[v0.1.0]: https://github.com/ChronoCoders/locproof/releases/tag/v0.1.0
