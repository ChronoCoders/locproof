use crate::signals::SignalSnapshot;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A signed attestation from one device covering its collected signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAttestation {
    /// Ed25519 public key of the attesting device.
    pub device_id: [u8; 32],
    /// Signals collected at the moment of attestation.
    pub signals: SignalSnapshot,
    /// Ed25519 signature over `bincode::serialize(signals)`.
    pub signature: Vec<u8>,
}

/// A proximity proof linking two device attestations, signed by the server.
///
/// The server's signature covers the entire proof (including `proximity_score`),
/// binding the score it computed to the underlying device attestations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProximityProof {
    pub id: Uuid,
    pub timestamp: i64,
    pub device_a: DeviceAttestation,
    pub device_b: DeviceAttestation,
    /// Proximity confidence in [0.0, 1.0], computed by the server.
    pub proximity_score: f64,
    /// Ed25519 public key of the issuing server.
    pub server_pubkey: [u8; 32],
    /// Ed25519 signature by the server over the proof's signing payload.
    pub server_signature: Vec<u8>,
}

/// Outcome of verifying a [`ProximityProof`].
#[derive(Debug, Clone)]
pub enum VerificationResult {
    /// All signatures and the timestamp window check out.
    Valid { score: f64 },
    /// A device or the server signature failed verification.
    /// `device` is `"a"`, `"b"`, or `"server"`.
    InvalidSignature { device: &'static str },
    /// The two device attestations were collected too far apart in time.
    TimestampMismatch { delta_seconds: i64 },
}
