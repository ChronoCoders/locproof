#![deny(warnings)]

pub mod proof;
pub mod scoring;
pub mod signals;
pub mod verify;

pub use proof::{DeviceAttestation, ProximityProof, VerificationResult};
pub use signals::{BarometerSignal, BleDevice, GpsSignal, SignalSnapshot};
