use serde::{Deserialize, Serialize};

/// GPS coordinates with accuracy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsSignal {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub accuracy_meters: f64,
    pub timestamp: i64,
}

/// Single BLE device detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleDevice {
    pub device_id: String,
    pub rssi: i8,
    pub name: Option<String>,
}

/// Barometric reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarometerSignal {
    pub pressure_hpa: f64,
    pub relative_altitude: Option<f64>,
    pub timestamp: i64,
}

/// All signals collected at a point in time.
///
/// `device_id` is the device's BLE-advertising identifier (MAC or UUID
/// string), used so the scoring algorithm can detect mutual visibility
/// in BLE scans. This is distinct from the Ed25519 public key on
/// [`crate::proof::DeviceAttestation`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalSnapshot {
    pub device_id: String,
    pub collected_at: i64,
    pub gps: Option<GpsSignal>,
    pub ble_devices: Vec<BleDevice>,
    pub barometer: Option<BarometerSignal>,
}
