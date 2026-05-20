# LocProof

**Tamper-proof digital witness.** Cryptographic evidence that two parties were
physically present at the same location. Court-admissible, blockchain-anchored,
impossible to fake.

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

All `/v1/*` endpoints require an API key in the `X-API-Key` header. `/health`
stays public. Generate a dev key and start the server:

```bash
export LOCPROOF_API_KEY="lp_live_$(openssl rand -hex 32)"
cargo run -p locproof-api
```

If `LOCPROOF_API_KEY` is unset the server starts in dev mode (no auth) and
prints a warning to stderr. Don't ship that.

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
