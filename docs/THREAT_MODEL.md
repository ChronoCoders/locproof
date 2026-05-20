# Threat Model

## Goals
A LocProof attestation proves two devices were within a bounded distance at a
specific time. We want this to hold against motivated adversaries who control one
or both endpoints.

## Attacker Capabilities

### GPS Spoofing
- **Difficulty**: Low. Apps like "Fake GPS Location" on rooted Android, SDR rigs
  (HackRF + gps-sdr-sim) for hardware spoofing.
- **Mitigation**: Cross-check with WiFi BSSIDs and BLE neighbors; require multiple
  uncorrelated signals; check GPS accuracy/HDOP and motion plausibility.

### WiFi AP Spoofing
- **Difficulty**: Medium. Requires broadcasting beacons with target BSSIDs; trivial
  with hostapd but requires colocation or replay infrastructure.
- **Mitigation**: Require *signal strength* correlation across both devices, not
  just AP presence. Weight long-lived, OUI-verified APs higher.

### Replay Attacks
- **Difficulty**: Low if nonces missing.
- **Mitigation**: Server-issued nonce per `ProofRequest`; timestamp bound; signed
  with device key.

### Collusion
- **Difficulty**: Low. Two cooperating devices can fabricate anything they like.
- **Mitigation**: **Fundamentally unsolvable** without external anchor. Mitigate by
  binding the proof to an externally observable signal (WiFi AP fingerprint of the
  area, validated against a crowdsourced database).

### Compromised SDK / Rooted Device
- **Difficulty**: Medium-High.
- **Mitigation**: iOS App Attest / Android Play Integrity API attestation included
  in proof payload. Detect at verification time.

## Out of Scope
- Proving a *single* device's location without a counterparty (use Apple/Google
  certified location services for that).
- Proving absence of proximity.
