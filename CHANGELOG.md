# Changelog

All notable changes to LocProof are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.5.0] — 2026-05-25

Phase 4b — Next.js dashboard. A separate app in `dashboard/` (Next.js 16
App Router, React 19, Tailwind v4, shadcn/ui, dark mode default) talking to
the 4a backend. Browser calls go through a same-origin `/api/*` rewrite so
the `lp_session` cookie flows under `SameSite=Strict`; server components
fetch the backend directly with the cookie forwarded.

### Added

- **Auth pages** — `/login` and `/register` (react-hook-form + zod),
  posting to `/auth/*`. On success the session cookie is set by the
  backend and the user lands on `/proofs`.
- **Authenticated shell** — `(dashboard)` route group with a server-side
  `requireSession()` guard, sidebar (nav, plan badge, sign-out), and a
  segment `error.tsx` using Next 16.2 `unstable_retry` to recover
  transient server-fetch failures.
- **`/proofs`** — cursor-paginated table ("Load more"), colored
  proximity-score badges, clickable rows opening a one-at-a-time JSON
  detail modal (request-token guarded against stale responses).
- **`/usage`** — current-month card (count, included quota, progress
  bar, with an Unlimited path for enterprise) and a Recharts bar chart
  of the last 12 months plus the current one.
- **`/billing`** — plan comparison cards mirroring the backend quotas,
  active plan highlighted; upgrade CTAs are disabled placeholders until
  Phase 4c (Stripe checkout).
- **`/settings/api-keys`** — list (name, status, created, last used)
  with a hide-inactive toggle (default on), a create flow that reveals
  the plaintext key exactly once (copy-to-clipboard, never persisted
  beyond modal state), and a guarded deactivate confirmation.
- **`GET /dashboard/proofs/:id`** — session-gated single-proof
  retrieval returning the full signed proof, backing the detail modal.

### Fixed

- **IDOR in proof retrieval.** `db::get_proof` fetched any proof by id
  with no tenant scoping, so `GET /v1/proofs/:id` let any customer key
  read another customer's proof. It is now scoped
  (`WHERE id = $1 AND customer_id = $2`); cross-customer access returns
  404 indistinguishably from a missing id. Both the `/v1` (API-key) and
  new `/dashboard` (session) handlers share `get_proof_impl`, and
  `api/tests/proofs_e2e.rs` adds a cross-customer 404 regression test.

## [v0.4.0] — 2026-05-20

Phase 4a — Backend foundation for the dashboard and billing work.

### Added

- **`api_keys` table** (migration 002). One customer can now hold many
  named keys (`lp_live_<32 hex>`), each with `last_used_at` and an
  `is_active` flag. Existing single-key customers are backfilled
  transparently — the old `customers.api_key_hash` column is dropped.
- **`plan` and `stripe_customer_id` on `customers`** (migration 002).
  `plan` defaults to `free`; `stripe_customer_id` is nullable. The
  plan→quota table lives in `api/src/plan.rs` (`free=100`,
  `starter=5000`, `growth=25000`, `enterprise=u32::MAX`); 4c will read
  it for submit-time quota enforcement.
- **`users` and `sessions` tables** (migration 003). Dashboard auth:
  email + argon2id password hash, server-side session tokens, 30-day
  sliding expiry capped at 90 days absolute.
- **`POST /auth/register | /auth/login | /auth/logout`**. Register
  creates the customer + user in a single transaction; if
  `STRIPE_SECRET_KEY` is set, a Stripe customer is created beforehand
  and persisted on `customers.stripe_customer_id`. Login returns the
  same `Unauthorized` for unknown-email and bad-password to avoid user
  enumeration. Email is lowercased + trimmed; passwords must be ≥ 12
  chars. Cookie `lp_session` is `HttpOnly`, `SameSite=Strict`, `Secure`
  in production, `Path=/`, `Max-Age=30 days`.
- **`require_user_session` middleware**. Reads the cookie, slides the
  expiry atomically (single `UPDATE ... FROM` JOIN against `users`),
  injects `user_id` and `CustomerId(Uuid)` (newtype, distinct from the
  bare `Uuid` injected by `require_customer_key`) into request
  extensions.
- **`GET /v1/proofs?cursor=&limit=`**. Cursor-paginated, newest first,
  customer-scoped. Cursor is opaque `base64url(<unix_micros>:<uuid>)`;
  malformed input returns 400 (no silent fall-through to page 1).
  `limit` defaults to 50, clamped to `[1, 200]`. `next_cursor` is
  `None` on the last page.
- **`GET /v1/usage`**. `{ plan, current_month: {month, count, quota},
  history: [{month, count}] }`. History is the prior 12 months oldest
  first; current month is fetched separately so a zero-count month
  still surfaces.
- **`GET /dashboard/proofs | /dashboard/usage`**. Same DB helpers as
  `/v1/*`, session-cookie auth instead of API key.
- **`POST /dashboard/keys`, `GET /dashboard/keys`, `DELETE /dashboard/keys/:id`**.
  Customer-scoped: the delete query filters on
  `WHERE id = $1 AND customer_id = $2` so cross-tenant probing returns
  404 indistinguishably from a non-existent id. List returns active and
  inactive keys (audit view).
- **`internal_err` helper everywhere DB / serde / crypto errors flow to
  500** (already in place from 3.x; reaffirmed across the new modules).
- **`api/tests/dashboard_e2e.rs`**. Full Postgres flow: register →
  mint key → use key on `/v1/proofs` → revoke → expect 401 on reuse →
  cross-tenant probe 404 → logout → stale cookie 401 → FK-safe
  cleanup.

### Changed

- `customers.api_key_hash` column dropped; auth middleware now joins
  `api_keys → customers` and requires both rows active.
- `customer::create` is now transactional (customer + first key in one
  tx). `customer::create_for_user` is the no-initial-key variant used
  by `/auth/register`.
- `routes/proofs.rs::list_impl` and `routes/usage.rs::get_usage_impl`
  are the pure bodies — both the `/v1` and `/dashboard` wrappers call
  into them, differing only in extractor.
- `AppState::new` gains `stripe: Option<stripe::Client>` and
  `cookie_secure: bool`. `cookie_secure = !dev_mode` in `main.rs`.
  If `STRIPE_SECRET_KEY` is unset at boot, registration leaves
  `stripe_customer_id` NULL and logs a warning.

### Security notes

- `/dashboard/*` has no rate limit in 4a. The existing limiter keys on
  `X-API-Key` (or the literal `"dev"` when missing), so cookie traffic
  would collapse into one bucket and block every dashboard user once
  any one of them hit the threshold. Per-session or per-IP limiting
  comes in a later phase.
- Register creates the Stripe customer *before* the DB transaction; a
  later DB failure orphans the Stripe customer. Acknowledged for now —
  a reconciliation pass will land alongside webhook handling in 4c.
- `list_keys` returns inactive keys so the dashboard can show a full
  audit view. The frontend can filter by `is_active` if desired.

### Not yet implemented

- Next.js dashboard UI (Phase 4b, `v0.5.0`).
- Stripe webhooks + plan upgrade/downgrade flow + submit-time quota
  enforcement (Phase 4c, `v0.6.0`).
- Per-session / per-IP rate limit.

## [v0.3.0] — 2026-05-20

Phase 3 — Proof persistence, retrieval, and usage counting.

### Added

- **`POST /v1/proofs` persists.** After signature verification and server
  signing, the full `ProximityProof` is written to the `proofs` table —
  hot columns (`id`, `customer_id`, `device_a_pubkey`, `device_b_pubkey`,
  `proximity_score`, `server_signature`) populated for indexed lookups,
  full proof stored as JSONB in `proof_data` for canonical client
  retrieval.
- **`GET /v1/proofs/:proof_id`** returns the stored `ProximityProof` as
  JSON, or `404 {"error": "proof not found"}` if no row matches. Replaces
  the Phase 1 placeholder that returned 501.
- **Per-customer usage counting.** Every successful `POST /v1/proofs`
  upserts the `usage` table for the current UTC month
  (`ON CONFLICT (customer_id, month) DO UPDATE SET proof_count = proof_count + 1`).
- **`customer_id` in request extensions.** `require_customer_key` now
  inserts the matched `customers.id` into the request extensions; handlers
  extract it via `Extension<Uuid>`.
- **`ApiError::NotFound(&'static str)` → 404** with a human-readable
  message.

### Changed

- Removed `ApiError::NotImplemented` — the only caller (the Phase 1 GET
  placeholder) now returns real data or 404.
- `sqlx` workspace dep now enables the `json` feature so JSONB columns
  map to `serde_json::Value`.
- Refreshed `.sqlx/` offline cache.

### Security notes

- The `POST` handler persists the proof and bumps usage as two separate
  awaits, not a single transaction. A mid-sequence failure leaves the
  proof stored but the usage count short by one. Acceptable at this
  phase; revisit if usage becomes a billing input.
- DB errors map to a generic 500 with no structured log. Add `tracing`
  before production traffic so 500s aren't silent.

### Not yet implemented

- Listing/pagination of a customer's proofs.
- Usage queries / monthly aggregation reporting (raw counts only).
- Integration tests against a throwaway Postgres.

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

[v0.4.0]: https://github.com/ChronoCoders/locproof/releases/tag/v0.4.0
[v0.3.0]: https://github.com/ChronoCoders/locproof/releases/tag/v0.3.0
[v0.2.0]: https://github.com/ChronoCoders/locproof/releases/tag/v0.2.0
[v0.1.0]: https://github.com/ChronoCoders/locproof/releases/tag/v0.1.0
