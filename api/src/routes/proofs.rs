use crate::db;
use crate::error::{internal_err, ApiError};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use locproof_core::{
    proof::{DeviceAttestation, ProximityProof},
    scoring::calculate_proximity_score,
    signals::SignalSnapshot,
    verify::{self, MAX_TIMESTAMP_DELTA_SECS},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_PAGE_LIMIT: i64 = 50;
const MAX_PAGE_LIMIT: i64 = 200;

#[derive(Deserialize)]
pub struct SubmitProofRequest {
    pub device_a: WireDeviceAttestation,
    pub device_b: WireDeviceAttestation,
}

#[derive(Deserialize)]
pub struct WireDeviceAttestation {
    #[serde(with = "b64")]
    pub device_id: Vec<u8>,
    pub signals: SignalSnapshot,
    #[serde(with = "b64")]
    pub signature: Vec<u8>,
}

#[derive(Serialize)]
pub struct ProofResponse {
    pub proof_id: Uuid,
    pub proximity_score: f64,
    pub verified: bool,
    pub timestamp: i64,
}

pub async fn submit(
    State(state): State<AppState>,
    Extension(customer_id): Extension<Uuid>,
    Json(req): Json<SubmitProofRequest>,
) -> Result<Json<ProofResponse>, ApiError> {
    let device_a = into_core_attestation(req.device_a, "a")?;
    let device_b = into_core_attestation(req.device_b, "b")?;

    if !verify::verify_attestation(&device_a) {
        return Err(ApiError::InvalidSignature("a"));
    }
    if !verify::verify_attestation(&device_b) {
        return Err(ApiError::InvalidSignature("b"));
    }

    let delta = (device_a.signals.collected_at - device_b.signals.collected_at).abs();
    if delta > MAX_TIMESTAMP_DELTA_SECS {
        return Err(ApiError::TimestampSkew {
            delta_seconds: delta,
            max: MAX_TIMESTAMP_DELTA_SECS,
        });
    }

    let proximity_score = calculate_proximity_score(&device_a.signals, &device_b.signals);
    let timestamp = chrono::Utc::now().timestamp();

    let mut proof = ProximityProof {
        id: Uuid::new_v4(),
        timestamp,
        device_a,
        device_b,
        proximity_score,
        server_pubkey: [0u8; 32],
        server_signature: Vec::new(),
    };

    verify::sign_proof(&mut proof, &state.server_keypair).map_err(internal_err)?;

    let mut tx = state.db.begin().await.map_err(internal_err)?;
    db::store_proof(&mut tx, customer_id, &proof)
        .await
        .map_err(internal_err)?;
    db::increment_usage(&mut tx, customer_id)
        .await
        .map_err(internal_err)?;
    tx.commit().await.map_err(internal_err)?;

    Ok(Json(ProofResponse {
        proof_id: proof.id,
        proximity_score: proof.proximity_score,
        verified: true,
        timestamp: proof.timestamp,
    }))
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct ProofSummary {
    pub proof_id: Uuid,
    pub proximity_score: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ListProofsResponse {
    pub proofs: Vec<ProofSummary>,
    /// `None` if this is the last page.
    pub next_cursor: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Extension(customer_id): Extension<Uuid>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListProofsResponse>, ApiError> {
    let limit = q
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .clamp(1, MAX_PAGE_LIMIT);
    let cursor = q.cursor.as_deref().map(decode_cursor).transpose()?;

    let rows = db::list_proofs(&state.db, customer_id, cursor, limit)
        .await
        .map_err(internal_err)?;

    let next_cursor = rows.last().and_then(|r| {
        if rows.len() as i64 == limit {
            Some(encode_cursor(r.created_at, r.id))
        } else {
            None
        }
    });

    Ok(Json(ListProofsResponse {
        proofs: rows
            .into_iter()
            .map(|r| ProofSummary {
                proof_id: r.id,
                proximity_score: r.proximity_score,
                created_at: r.created_at,
            })
            .collect(),
        next_cursor,
    }))
}

/// Cursor encoding: `base64url(<unix_micros>:<uuid>)`. Opaque to clients;
/// they only round-trip the string from `next_cursor` back into the next
/// request's `?cursor=`.
fn encode_cursor(ts: DateTime<Utc>, id: Uuid) -> String {
    URL_SAFE_NO_PAD.encode(format!("{}:{}", ts.timestamp_micros(), id))
}

fn decode_cursor(s: &str) -> Result<(DateTime<Utc>, Uuid), ApiError> {
    let bad = || ApiError::BadRequest("invalid cursor".into());
    let bytes = URL_SAFE_NO_PAD.decode(s).map_err(|_| bad())?;
    let decoded = std::str::from_utf8(&bytes).map_err(|_| bad())?;
    let (ts_str, id_str) = decoded.split_once(':').ok_or_else(bad)?;
    let micros: i64 = ts_str.parse().map_err(|_| bad())?;
    let ts = DateTime::<Utc>::from_timestamp_micros(micros).ok_or_else(bad)?;
    let id: Uuid = id_str.parse().map_err(|_| bad())?;
    Ok((ts, id))
}

pub async fn get_proof(
    State(state): State<AppState>,
    Path(proof_id): Path<Uuid>,
) -> Result<Json<ProximityProof>, ApiError> {
    db::get_proof(&state.db, proof_id)
        .await
        .map_err(internal_err)?
        .map(Json)
        .ok_or(ApiError::NotFound("proof"))
}

fn into_core_attestation(
    wire: WireDeviceAttestation,
    label: &'static str,
) -> Result<DeviceAttestation, ApiError> {
    let device_id: [u8; 32] =
        wire.device_id.as_slice().try_into().map_err(|_| {
            ApiError::BadRequest(format!("device {label}: device_id must be 32 bytes"))
        })?;
    Ok(DeviceAttestation {
        device_id,
        signals: wire.signals,
        signature: wire.signature,
    })
}

mod b64 {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}
