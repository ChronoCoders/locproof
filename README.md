# LocProof

**Tamper-resistant digital witness.** Cryptographic evidence that two parties
were physically present at the same location. Strongly resistant to spoofing
when device attestation and multi-signal correlation are enforced. Not immune to
sophisticated collusion or advanced relay attacks without additional external
anchors.

## Use Cases
- **Logistics** — chain-of-custody handoffs between drivers, warehouses, recipients.
- **Insurance** — verify claimant presence at incident location.
- **Legal** — witness/notary/contract-signing presence attestation.

## How It Works
1. Your server requests a proof via the LocProof API; we return a single-use nonce.
2. Two devices, each running the LocProof SDK, collect signals (GPS, WiFi, BLE,
   barometer) and sign their observations with their Ed25519 device keys.
3. LocProof correlates the observations, computes a proximity score, and returns a
   signed `ProximityProof` that your system (or a counterparty) can verify offline.

## Pricing
Per-proof, Twilio-style. Volume discounts.

## Running Locally

LocProof has two key types:

- **Bootstrap admin key** (`lp_admin_<32 hex>`) — set via `LOCPROOF_API_KEY`.
  Gates `/admin/*` only. Used to mint and manage customers.
- **Customer keys** (`lp_live_<32 hex>`) — minted via `POST /admin/customers`,
  stored as SHA-256 hashes. Gate `/v1/*`. The plaintext is returned exactly
  once at creation.

`/health` is public.

```bash
# 1. Set the bootstrap admin key and start the server.
export LOCPROOF_API_KEY="lp_admin_$(openssl rand -hex 16)"
cargo run -p locproof-api

# 2. Mint a customer key.
curl -sX POST http://localhost:3000/admin/customers \
  -H "X-API-Key: $LOCPROOF_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name":"acme"}'
# → {"id":"…","name":"acme","api_key":"lp_live_…","created_at":"…"}

# 3. Use the returned customer key on /v1/*.
curl -X POST http://localhost:3000/v1/proofs \
  -H "X-API-Key: lp_live_…" -d @proof.json
```

If `LOCPROOF_API_KEY` is unset, the server requires `LOCPROOF_DEV=1` and
starts with `/admin/*` open (no admin auth). `/v1/*` always enforces a real
customer key — mint one via the open `/admin/customers` in dev. Don't ship
dev mode.

The server signing keypair is persisted at `data/server.key` (mode `0600`,
gitignored). Delete it and restart to rotate.

## Roadmap

### Phase 1 (Current) — API Foundation
- Core proof types + scoring ✓
- PostgreSQL storage ✓
- API auth + rate limiting ✓
- Customer management (in progress)

### Phase 2 — Mobile SDKs
- iOS SDK (Swift)
- Android SDK (Kotlin)
- Signal collection (BLE, GPS, Barometer)

### Phase 3 — Blockchain Anchoring
- Anchor proof hashes to Base L2
- EIP-712 typed signatures
- Batch anchoring with Merkle trees
- On-chain verification contract

### Phase 4 — Dashboard + Billing
- Next.js customer dashboard
- Stripe billing integration
- Usage analytics

### Phase 5 — Production Launch
- Akamai/Vultr deployment
- Monitoring + alerting
- Documentation site

## Status
Early development. Protocol spec in `docs/PROTOCOL.md`.

## License
[BUSL 1.1](./LICENSE). Source-available; commercial production use requires a
license from ChronoCoders.
