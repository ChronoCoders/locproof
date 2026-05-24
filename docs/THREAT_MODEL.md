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

### Relay Attacks on BLE
- **Description**: An attacker relays BLE signals between two devices that are
  *not* physically present to each other, making them appear adjacent. Frames are
  captured near device A, tunneled over a fast out-of-band link, and replayed near
  device B (and vice versa) — a wormhole/relay, as documented against BLE passive
  car entry.
- **Difficulty**: Medium-High. Requires relay hardware at both ends and
  low-latency tunneling, but the technique is well understood.
- **Current mitigation**: RSSI correlation between the two devices' observations.
  **Partial, not sufficient alone** — a relay can attenuate or shape RSSI, so it
  cannot be relied on by itself.
- **Planned mitigation**: Ultra-wideband (UWB) time-of-flight ranging where the
  hardware supports it (sub-nanosecond ToF resists relays, which necessarily add
  latency); mutual RSSI consistency checks requiring both directions to agree
  within a plausibility envelope.

### Replay Attacks
- **Difficulty**: Low if nonces missing.
- **Mitigation**: Server-issued nonce per `ProofRequest`; timestamp bound; signed
  with device key.

### Collusion
- **Difficulty**: Low. Two cooperating devices under one party's control can
  fabricate a proof for any time and place — they simply exchange or synthesize
  matching signals.
- **Honest assessment**: This is the hardest threat and is **not fully solvable**
  at the protocol level without a trusted external anchor. LocProof *reduces* but
  does not *eliminate* collusion risk. A proof should be treated as one piece of
  evidence among several, never as standalone proof of presence — which is why we
  qualify its evidentiary value rather than claim it is impossible to fake.
- **Current mitigation**: Bind the proof to externally observable signals (e.g.
  WiFi AP fingerprints of the area) validated against a crowdsourced database, so
  colluding devices must also reproduce a plausible, third-party-observable
  environment.
- **Planned mitigation**: Device attestation (below) to raise the cost of running
  modified or emulated clients; anomaly detection on submission patterns; optional
  anchoring to third-party signals the colluders do not control.

### Compromised SDK / Rooted or Jailbroken Device
- **Difficulty**: Medium-High.
- **Risk**: A rooted Android or jailbroken iOS device can run a modified SDK that
  fabricates or replays signals, bypasses real sensor collection, or extracts
  device keys — undermining the integrity of every signal a proof depends on.
- **Current mitigation**: None enforced today; collected signals are trusted as
  submitted.
- **Planned mitigation**: iOS App Attest / Android Play Integrity attestation
  embedded in the proof payload and **enforced** at verification time — proofs
  lacking a valid attestation are rejected, not merely flagged.

## Out of Scope
- Proving a *single* device's location without a counterparty (use Apple/Google
  certified location services for that).
- Proving absence of proximity.
