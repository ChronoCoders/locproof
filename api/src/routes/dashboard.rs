//! `/dashboard/*` endpoints. Session-cookie auth via `require_user_session`;
//! every handler is scoped to the session's `CustomerId`. Same underlying
//! DB helpers as `/v1/*` — these are thin shells over `proofs::list_impl`,
//! `usage::get_usage_impl`, and the `api_key` model.

use crate::auth::CustomerId;
use crate::error::{internal_err, ApiError};
use crate::models::api_key;
use crate::routes;
use crate::routes::{
    proofs::{ListProofsResponse, ListQuery},
    usage::UsageResponse,
};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use locproof_core::proof::ProximityProof;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub async fn list_proofs(
    State(s): State<AppState>,
    Extension(CustomerId(customer_id)): Extension<CustomerId>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListProofsResponse>, ApiError> {
    routes::proofs::list_impl(&s, customer_id, q).await
}

pub async fn get_proof(
    State(s): State<AppState>,
    Extension(CustomerId(customer_id)): Extension<CustomerId>,
    Path(proof_id): Path<Uuid>,
) -> Result<Json<ProximityProof>, ApiError> {
    routes::proofs::get_proof_impl(&s, customer_id, proof_id).await
}

pub async fn get_usage(
    State(s): State<AppState>,
    Extension(CustomerId(customer_id)): Extension<CustomerId>,
) -> Result<Json<UsageResponse>, ApiError> {
    routes::usage::get_usage_impl(&s, customer_id).await
}

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub id: Uuid,
    pub name: String,
    /// Plaintext key — returned exactly once.
    pub api_key: String,
    pub created_at: DateTime<Utc>,
}

pub async fn create_key(
    State(s): State<AppState>,
    Extension(CustomerId(customer_id)): Extension<CustomerId>,
    Json(req): Json<CreateKeyRequest>,
) -> Result<(StatusCode, Json<CreateKeyResponse>), ApiError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("name must not be empty".into()));
    }
    let mut conn = s.db.acquire().await.map_err(internal_err)?;
    let (row, plaintext) = api_key::create(&mut conn, customer_id, name)
        .await
        .map_err(internal_err)?;
    Ok((
        StatusCode::CREATED,
        Json(CreateKeyResponse {
            id: row.id,
            name: row.name,
            api_key: plaintext,
            created_at: row.created_at,
        }),
    ))
}

#[derive(Serialize)]
pub struct KeySummary {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

pub async fn list_keys(
    State(s): State<AppState>,
    Extension(CustomerId(customer_id)): Extension<CustomerId>,
) -> Result<Json<Vec<KeySummary>>, ApiError> {
    let rows = api_key::list_for_customer(&s.db, customer_id)
        .await
        .map_err(internal_err)?;
    Ok(Json(
        rows.into_iter()
            .map(|k| KeySummary {
                id: k.id,
                name: k.name,
                created_at: k.created_at,
                last_used_at: k.last_used_at,
                is_active: k.is_active,
            })
            .collect(),
    ))
}

pub async fn delete_key(
    State(s): State<AppState>,
    Extension(CustomerId(customer_id)): Extension<CustomerId>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let matched = api_key::deactivate_for_customer(&s.db, id, customer_id)
        .await
        .map_err(internal_err)?;
    if matched {
        Ok(StatusCode::OK)
    } else {
        Err(ApiError::NotFound("api_key"))
    }
}
