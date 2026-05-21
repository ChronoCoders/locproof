use crate::error::{internal_err, ApiError};
use crate::models::customer;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateCustomerRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct CreateCustomerResponse {
    pub id: Uuid,
    pub name: String,
    /// Plaintext API key. Returned exactly once, at creation time.
    pub api_key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct CustomerSummary {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

pub async fn create_customer(
    State(state): State<AppState>,
    Json(req): Json<CreateCustomerRequest>,
) -> Result<(StatusCode, Json<CreateCustomerResponse>), ApiError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("name must not be empty".into()));
    }
    let (row, api_key) = customer::create(&state.db, name)
        .await
        .map_err(internal_err)?;
    Ok((
        StatusCode::CREATED,
        Json(CreateCustomerResponse {
            id: row.id,
            name: row.name,
            api_key,
            created_at: row.created_at,
        }),
    ))
}

pub async fn list_customers(
    State(state): State<AppState>,
) -> Result<Json<Vec<CustomerSummary>>, ApiError> {
    let rows = customer::list(&state.db)
        .await
        .map_err(internal_err)?;
    let out = rows
        .into_iter()
        .map(|c| CustomerSummary {
            id: c.id,
            name: c.name,
            created_at: c.created_at,
            is_active: c.is_active,
        })
        .collect();
    Ok(Json(out))
}

pub async fn delete_customer(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    customer::deactivate(&state.db, id)
        .await
        .map_err(internal_err)?;
    Ok(StatusCode::OK)
}
