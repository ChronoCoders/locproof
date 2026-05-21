use crate::signals::{BarometerSignal, BleDevice, GpsSignal, SignalSnapshot};

/// Calculate proximity score from two signal snapshots.
///
/// Returns a value in `[0.0, 1.0]`: 0.0 = no evidence of proximity, 1.0 =
/// strong evidence. Weighted blend: BLE 0.5, barometer 0.3, GPS 0.2.
/// Weights are renormalised over only the signal classes that produced a
/// value, so missing signals don't penalise the score.
pub fn calculate_proximity_score(a: &SignalSnapshot, b: &SignalSnapshot) -> f64 {
    let mut score = 0.0;
    let mut weight_sum = 0.0;

    if let Some(ble_score) =
        calculate_ble_proximity(&a.ble_devices, &b.ble_devices, &a.device_id, &b.device_id)
    {
        score += ble_score * 0.5;
        weight_sum += 0.5;
    }

    if let Some(baro_score) = calculate_barometer_proximity(&a.barometer, &b.barometer) {
        score += baro_score * 0.3;
        weight_sum += 0.3;
    }

    if let Some(gps_score) = calculate_gps_proximity(&a.gps, &b.gps) {
        score += gps_score * 0.2;
        weight_sum += 0.2;
    }

    if weight_sum > 0.0 {
        score / weight_sum
    } else {
        0.0
    }
}

/// BLE mutual-visibility score.
///
/// Looks for each device's advertised id in the other's BLE scan. If both
/// directions saw each other, average the two RSSIs. If only one did, use
/// that single reading. Returns `None` when neither side saw the other.
fn calculate_ble_proximity(
    a_devices: &[BleDevice],
    b_devices: &[BleDevice],
    a_id: &str,
    b_id: &str,
) -> Option<f64> {
    let a_sees_b = a_devices.iter().find(|d| d.device_id == b_id);
    let b_sees_a = b_devices.iter().find(|d| d.device_id == a_id);

    match (a_sees_b, b_sees_a) {
        (Some(ab), Some(ba)) => {
            let avg_rssi = (f64::from(ab.rssi) + f64::from(ba.rssi)) / 2.0;
            Some(rssi_to_score(avg_rssi))
        }
        (Some(ab), None) => Some(rssi_to_score(f64::from(ab.rssi))),
        (None, Some(ba)) => Some(rssi_to_score(f64::from(ba.rssi))),
        (None, None) => None,
    }
}

/// Convert an RSSI value (dBm) to a `[0.0, 1.0]` proximity score.
///
/// `>= -40 dBm` → 1.0 (very close, <0.5 m); `<= -100 dBm` → 0.0 (out of
/// range or noise); linear between.
fn rssi_to_score(rssi: f64) -> f64 {
    if rssi >= -40.0 {
        1.0
    } else if rssi <= -100.0 {
        0.0
    } else {
        (rssi + 100.0) / 60.0
    }
}

/// Barometer-based altitude-agreement score.
///
/// Pressure differences map to vertical separation at roughly 0.12 hPa/m.
/// `< 0.4 hPa` (same floor, ~3 m) → 1.0; falls linearly to 0.0 at 1.2 hPa
/// (~10 m vertical).
fn calculate_barometer_proximity(
    a: &Option<BarometerSignal>,
    b: &Option<BarometerSignal>,
) -> Option<f64> {
    let (baro_a, baro_b) = match (a, b) {
        (Some(a), Some(b)) => (a, b),
        _ => return None,
    };

    let pressure_diff = (baro_a.pressure_hpa - baro_b.pressure_hpa).abs();
    let score = if pressure_diff < 0.4 {
        1.0
    } else if pressure_diff < 1.2 {
        1.0 - ((pressure_diff - 0.4) / 0.8)
    } else {
        0.0
    };
    Some(score.clamp(0.0, 1.0))
}

/// GPS-distance score with accuracy compensation.
///
/// The reported GPS accuracy of the worse fix is subtracted from the
/// haversine distance before scoring, so two devices reporting overlapping
/// uncertainty bubbles are treated as adjacent. `<= 10 m` effective
/// distance → 1.0; `>= 100 m` → 0.0; linear between.
fn calculate_gps_proximity(a: &Option<GpsSignal>, b: &Option<GpsSignal>) -> Option<f64> {
    let (gps_a, gps_b) = match (a, b) {
        (Some(a), Some(b)) => (a, b),
        _ => return None,
    };

    let distance = haversine_meters(
        gps_a.latitude,
        gps_a.longitude,
        gps_b.latitude,
        gps_b.longitude,
    );
    let min_accuracy = gps_a.accuracy_meters.max(gps_b.accuracy_meters);
    let effective_distance = (distance - min_accuracy).max(0.0);

    let score = if effective_distance <= 10.0 {
        1.0
    } else if effective_distance >= 100.0 {
        0.0
    } else {
        1.0 - ((effective_distance - 10.0) / 90.0)
    };
    Some(score)
}

/// Great-circle distance between two GPS coordinates, in meters.
fn haversine_meters(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_000.0;

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_M * c
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(device_id: &str) -> SignalSnapshot {
        SignalSnapshot {
            device_id: device_id.to_string(),
            collected_at: 0,
            gps: None,
            ble_devices: vec![],
            barometer: None,
        }
    }

    fn ble(id: &str, rssi: i8) -> BleDevice {
        BleDevice {
            device_id: id.to_string(),
            rssi,
            name: None,
        }
    }

    #[test]
    fn no_signals_scores_zero() {
        assert_eq!(
            calculate_proximity_score(&snapshot("a"), &snapshot("b")),
            0.0
        );
    }

    #[test]
    fn test_rssi_to_score() {
        assert_eq!(rssi_to_score(-30.0), 1.0);
        assert_eq!(rssi_to_score(-40.0), 1.0);
        assert!((rssi_to_score(-70.0) - 0.5).abs() < 0.01);
        assert_eq!(rssi_to_score(-100.0), 0.0);
        assert_eq!(rssi_to_score(-120.0), 0.0);
    }

    #[test]
    fn test_haversine() {
        let d = haversine_meters(40.7128, -74.0060, 40.7138, -74.0060);
        assert!(d > 100.0 && d < 120.0, "got {d}");
    }

    #[test]
    fn test_barometer_same_floor() {
        let a = Some(BarometerSignal {
            pressure_hpa: 1013.25,
            relative_altitude: None,
            timestamp: 0,
        });
        let b = Some(BarometerSignal {
            pressure_hpa: 1013.30,
            relative_altitude: None,
            timestamp: 0,
        });
        let score = calculate_barometer_proximity(&a, &b).expect("score");
        assert!(score > 0.9);
    }

    #[test]
    fn ble_mutual_visibility_strong_rssi() {
        let mut a = snapshot("device-a");
        let mut b = snapshot("device-b");
        a.ble_devices = vec![ble("device-b", -35)];
        b.ble_devices = vec![ble("device-a", -40)];
        let s = calculate_proximity_score(&a, &b);
        assert!(
            (s - 1.0).abs() < 1e-9,
            "mutual visibility with very strong RSSI should saturate at 1.0, got {s}"
        );
    }

    #[test]
    fn ble_one_way_visibility_still_scores() {
        let mut a = snapshot("device-a");
        let b = snapshot("device-b");
        a.ble_devices = vec![ble("device-b", -55)];
        let s = calculate_proximity_score(&a, &b);
        assert!(s > 0.7, "one-way RSSI -55 should score ~0.75, got {s}");
    }

    #[test]
    fn ble_no_mutual_visibility_skipped() {
        let mut a = snapshot("device-a");
        let mut b = snapshot("device-b");
        a.ble_devices = vec![ble("some-other-beacon", -40)];
        b.ble_devices = vec![ble("yet-another", -40)];
        assert_eq!(calculate_proximity_score(&a, &b), 0.0);
    }

    #[test]
    fn gps_accuracy_compensation() {
        let mut a = snapshot("device-a");
        let mut b = snapshot("device-b");
        a.gps = Some(GpsSignal {
            latitude: 40.7128,
            longitude: -74.0060,
            altitude: None,
            accuracy_meters: 30.0,
            timestamp: 0,
        });
        b.gps = Some(GpsSignal {
            latitude: 40.7130,
            longitude: -74.0060,
            altitude: None,
            accuracy_meters: 30.0,
            timestamp: 0,
        });
        let s = calculate_proximity_score(&a, &b);
        assert!(
            s > 0.9,
            "raw ~22m minus 30m accuracy → effective 0, got {s}"
        );
    }
}
