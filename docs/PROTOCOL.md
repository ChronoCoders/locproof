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
2. Check timestamps within tolerance (e.g., 30 seconds)
3. Compare signals for correlation
4. Calculate proximity score

## Open Questions
- Minimum signal combination for reliable proof?
- Offline proof generation flow?
- How to handle one device being a phone, other being IoT?

## Next Steps
- Build iOS/Android test app to collect real signal data
- Test correlation accuracy at various distances
- Define scoring algorithm based on real data
