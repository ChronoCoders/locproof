# Mobile Signal Availability

## iOS

### GPS (CoreLocation)
- CLLocationManager
- Permissions: NSLocationWhenInUseUsageDescription, NSLocationAlwaysUsageDescription
- Background: requires "location" background mode
- Accuracy: kCLLocationAccuracyBest (GPS), kCLLocationAccuracyNearestTenMeters
- Returns: latitude, longitude, altitude, horizontalAccuracy, verticalAccuracy, timestamp

### WiFi (Limited)
- NEHotspotHelper: requires Apple entitlement (hard to get)
- CNCopyCurrentNetworkInfo: only current connected network, deprecated iOS 13+
- Workaround: none for scanning nearby APs without entitlement
- iOS restriction: cannot scan WiFi networks freely

### Bluetooth (CoreBluetooth)
- CBCentralManager: scan for BLE peripherals
- CBPeripheralManager: advertise as peripheral
- RSSI available during scan
- No permissions popup until actual scan
- Background: limited, requires "bluetooth-central" mode

### UWB (Nearby Interaction)
- NISession: cm-level ranging with U1 chip
- Only iPhone 11+ and Apple Watch Series 6+
- Requires both devices to run compatible app
- Best accuracy but limited device support

### Barometer (CMAltimeter)
- Relative altitude changes
- Not absolute altitude
- Permission: none required
- Available iPhone 6+

## Android

### GPS (LocationManager / FusedLocationProvider)
- FusedLocationProviderClient preferred
- Permissions: ACCESS_FINE_LOCATION, ACCESS_COARSE_LOCATION
- Background: ACCESS_BACKGROUND_LOCATION (Android 10+)
- Returns: latitude, longitude, altitude, accuracy, bearing, speed

### WiFi (WifiManager)
- WifiManager.getScanResults(): list of nearby APs
- Returns: SSID, BSSID, RSSI, frequency, capabilities
- Permission: ACCESS_FINE_LOCATION required for scan
- Throttling: Android 9+ limits scans to 4 per 2 minutes

### Bluetooth (BluetoothAdapter)
- BluetoothLeScanner: scan BLE devices
- Returns: device address, name, RSSI
- Permission: BLUETOOTH_SCAN (Android 12+)
- No throttling like WiFi

### UWB (Android UWB API)
- UwbManager: ranging sessions
- Limited devices: Pixel 6 Pro+, Samsung Galaxy S21+
- Requires both devices to support UWB

### Barometer (SensorManager)
- TYPE_PRESSURE sensor
- Absolute pressure in hPa
- Can calculate altitude with formula
- Permission: none required

## Signal Reliability Matrix

| Signal    | iOS Available | Android Available | Spoofable | Indoor | Accuracy |
|-----------|---------------|-------------------|-----------|--------|----------|
| GPS       | Yes           | Yes               | Easy      | Poor   | 3-5m     |
| WiFi Scan | No*           | Yes               | Medium    | Good   | Room     |
| BLE RSSI  | Yes           | Yes               | Medium    | Good   | 1-5m     |
| UWB       | Limited       | Limited           | Hard      | Good   | 10cm     |
| Barometer | Yes           | Yes               | Hard      | Good   | Floor    |

*iOS WiFi scanning requires special Apple entitlement

## Recommended Approach
Primary: BLE RSSI (both platforms, reasonable accuracy)
Secondary: Barometer (altitude correlation)
Tertiary: GPS (outdoor only, cross-check)

WiFi scanning not viable for iOS without entitlement.
UWB ideal but device support too limited for B2B product.

## SDK Implementation Priority
1. BLE ranging between two devices
2. Barometric pressure comparison
3. GPS as supplementary signal
4. UWB as optional enhancement for supported devices
