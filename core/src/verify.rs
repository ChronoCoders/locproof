use crate::proof::{DeviceAttestation, ProximityProof, VerificationResult};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::Serialize;
use uuid::Uuid;

/// Maximum allowed difference between the two devices' `collected_at`
/// timestamps for a proof to be considered fresh.
pub const MAX_TIMESTAMP_DELTA_SECS: i64 = 30;

/// Canonical signing payload for a [`ProximityProof`].
///
/// Excludes `server_signature` (the field being computed) but includes
/// `server_pubkey` so a signature cannot be reattributed to another key.
#[derive(Serialize)]
struct ProofSigningPayload<'a> {
    id: &'a Uuid,
    timestamp: i64,
    device_a: &'a DeviceAttestation,
    device_b: &'a DeviceAttestation,
    proximity_score: f64,
    server_pubkey: &'a [u8; 32],
}

/// Verify a proximity proof end-to-end.
///
/// Checks, in order:
/// 1. Device A's signature over its signal snapshot.
/// 2. Device B's signature over its signal snapshot.
/// 3. That both snapshots were collected within `MAX_TIMESTAMP_DELTA_SECS` of each other.
/// 4. The server's signature over the full proof (binding the proximity score).
pub fn verify_proof(proof: &ProximityProof) -> VerificationResult {
    if !verify_attestation(&proof.device_a) {
        return VerificationResult::InvalidSignature { device: "a" };
    }
    if !verify_attestation(&proof.device_b) {
        return VerificationResult::InvalidSignature { device: "b" };
    }

    let delta = (proof.device_a.signals.collected_at - proof.device_b.signals.collected_at).abs();
    if delta > MAX_TIMESTAMP_DELTA_SECS {
        return VerificationResult::TimestampMismatch {
            delta_seconds: delta,
        };
    }

    if !verify_server_signature(proof) {
        return VerificationResult::InvalidSignature { device: "server" };
    }

    VerificationResult::Valid {
        score: proof.proximity_score,
    }
}

/// Sign a proof with the server's keypair, populating `server_pubkey` and
/// `server_signature`. Call this after computing `proximity_score` and after
/// both device attestations have been verified.
pub fn sign_proof(
    proof: &mut ProximityProof,
    server_keypair: &SigningKey,
) -> Result<(), bincode::Error> {
    proof.server_pubkey = server_keypair.verifying_key().to_bytes();
    let payload = ProofSigningPayload {
        id: &proof.id,
        timestamp: proof.timestamp,
        device_a: &proof.device_a,
        device_b: &proof.device_b,
        proximity_score: proof.proximity_score,
        server_pubkey: &proof.server_pubkey,
    };
    let bytes = bincode::serialize(&payload)?;
    let sig = server_keypair.sign(&bytes);
    proof.server_signature = sig.to_bytes().to_vec();
    Ok(())
}

/// Verify a single device attestation's signature over its signal snapshot.
///
/// Returns `true` iff `device_id` is a well-formed Ed25519 public key,
/// `signature` is 64 bytes, and the signature is valid over the bincode
/// encoding of `signals`.
pub fn verify_attestation(att: &DeviceAttestation) -> bool {
    let Ok(vk) = VerifyingKey::from_bytes(&att.device_id) else {
        return false;
    };
    let Ok(sig_bytes): Result<[u8; 64], _> = att.signature.as_slice().try_into() else {
        return false;
    };
    let sig = Signature::from_bytes(&sig_bytes);
    let Ok(msg) = bincode::serialize(&att.signals) else {
        return false;
    };
    vk.verify(&msg, &sig).is_ok()
}

fn verify_server_signature(proof: &ProximityProof) -> bool {
    let Ok(vk) = VerifyingKey::from_bytes(&proof.server_pubkey) else {
        return false;
    };
    let Ok(sig_bytes): Result<[u8; 64], _> = proof.server_signature.as_slice().try_into() else {
        return false;
    };
    let sig = Signature::from_bytes(&sig_bytes);
    let payload = ProofSigningPayload {
        id: &proof.id,
        timestamp: proof.timestamp,
        device_a: &proof.device_a,
        device_b: &proof.device_b,
        proximity_score: proof.proximity_score,
        server_pubkey: &proof.server_pubkey,
    };
    let Ok(msg) = bincode::serialize(&payload) else {
        return false;
    };
    vk.verify(&msg, &sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::SignalSnapshot;
    use rand::rngs::OsRng;

    fn sign_attestation(key: &SigningKey, snapshot: SignalSnapshot) -> DeviceAttestation {
        let msg = bincode::serialize(&snapshot).expect("serialize snapshot");
        let sig = key.sign(&msg);
        DeviceAttestation {
            device_id: key.verifying_key().to_bytes(),
            signals: snapshot,
            signature: sig.to_bytes().to_vec(),
        }
    }

    fn snapshot_at(ts: i64) -> SignalSnapshot {
        SignalSnapshot {
            device_id: "test-device".to_string(),
            collected_at: ts,
            gps: None,
            ble_devices: vec![],
            barometer: None,
        }
    }

    fn signed_proof(score: f64, ts_a: i64, ts_b: i64) -> (ProximityProof, SigningKey) {
        let key_a = SigningKey::generate(&mut OsRng);
        let key_b = SigningKey::generate(&mut OsRng);
        let server = SigningKey::generate(&mut OsRng);
        let mut proof = ProximityProof {
            id: Uuid::new_v4(),
            timestamp: 1_700_000_000,
            device_a: sign_attestation(&key_a, snapshot_at(ts_a)),
            device_b: sign_attestation(&key_b, snapshot_at(ts_b)),
            proximity_score: score,
            server_pubkey: [0u8; 32],
            server_signature: vec![],
        };
        sign_proof(&mut proof, &server).expect("server sign");
        (proof, server)
    }

    #[test]
    fn valid_proof_verifies() {
        let (proof, _) = signed_proof(0.9, 1_700_000_000, 1_700_000_010);
        assert!(matches!(
            verify_proof(&proof),
            VerificationResult::Valid { .. }
        ));
    }

    #[test]
    fn tampered_device_signature_fails() {
        let (mut proof, _) = signed_proof(0.9, 1_700_000_000, 1_700_000_000);
        proof.device_a.signals.collected_at = 1_700_000_999;
        assert!(matches!(
            verify_proof(&proof),
            VerificationResult::InvalidSignature { device: "a" }
        ));
    }

    #[test]
    fn timestamp_skew_rejected() {
        let (proof, _) = signed_proof(0.9, 1_700_000_000, 1_700_000_100);
        assert!(matches!(
            verify_proof(&proof),
            VerificationResult::TimestampMismatch { .. }
        ));
    }

    #[test]
    fn tampered_score_fails_server_signature() {
        let (mut proof, _) = signed_proof(0.9, 1_700_000_000, 1_700_000_000);
        proof.proximity_score = 0.99;
        assert!(matches!(
            verify_proof(&proof),
            VerificationResult::InvalidSignature { device: "server" }
        ));
    }

    #[test]
    fn forged_server_pubkey_fails() {
        let (mut proof, _) = signed_proof(0.9, 1_700_000_000, 1_700_000_000);
        let attacker = SigningKey::generate(&mut OsRng);
        proof.server_pubkey = attacker.verifying_key().to_bytes();
        assert!(matches!(
            verify_proof(&proof),
            VerificationResult::InvalidSignature { device: "server" }
        ));
    }
}
