# Proximity Proof Protocol

## Overview
LocProof proves two devices were physically close at a specific time.

## Problem
- Device A claims it was near Device B at time T
- Need cryptographic proof without central authority
- Must resist location spoofing

## Available Signals

### GPS
- Accuracy: 3-5m outdoor, poor indoor
- iOS: CoreLocation
- Android: LocationManager
- Limitation: easily spoofed with apps

### WiFi
- Nearby access points + signal strength (RSSI)
- iOS: NEHotspotHelper (limited, requires entitlement)
- Android: WifiManager.getScanResults()
- Correlation: same APs with similar RSSI = likely nearby

### Bluetooth Low Energy
- Device-to-device ranging
- iOS: CoreBluetooth, iBeacon
- Android: BluetoothAdapter
- UWB (U1 chip): cm-level accuracy but limited devices

### Barometric Pressure
- Altitude correlation
- iOS: CMAltimeter
- Android: SensorManager (TYPE_PRESSURE)
- Same pressure = same altitude (within building floor)

## Proof Structure (Draft)
```rust
struct ProximityProof {
    id: Uuid,
    timestamp: i64,
    device_a: DeviceAttestation,
    device_b: DeviceAttestation,
    proximity_score: f64,  // 0.0 to 1.0
    proof_signature: Vec<u8>,
}

struct DeviceAttestation {
    device_id: PublicKey,
    signals: SignalData,
    collected_at: i64,
    signature: Vec<u8>,
}
```

## Verification Algorithm
1. Verify both device signatures
2. Verify each device's platform attestation (see below)
3. Check timestamps within tolerance (e.g., 30 seconds)
4. Compare signals for correlation
5. Calculate proximity score

## Device Attestation Requirements
Signals are only trustworthy if they come from a genuine, unmodified app on a
genuine device. Each `DeviceAttestation` therefore carries a platform
attestation token, which is **mandatory**:

- **iOS** — Apple **App Attest** is mandatory. The SDK generates a hardware-backed
  attestation key and includes an App Attest assertion over the signed signal
  payload.
- **Android** — Google **Play Integrity** is mandatory. The SDK includes a Play
  Integrity verdict bound to the same payload.

Proofs from unattested devices **must be rejected, not flagged** — verification
fails closed. A missing, malformed, or invalid attestation (including
rooted/jailbroken or emulator verdicts) causes the entire proof to be rejected;
it is never accepted with a lowered score or a warning annotation.

## Open Questions
- Minimum signal combination for reliable proof?
- Offline proof generation flow?
- How to handle one device being a phone, other being IoT?

## Next Steps
- Build iOS/Android test app to collect real signal data
- Test correlation accuracy at various distances
- Define scoring algorithm based on real data
