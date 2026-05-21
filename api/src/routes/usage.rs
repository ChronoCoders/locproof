use crate::error::{internal_err, ApiError};
use crate::models::customer;
use crate::state::AppState;
use crate::{db, plan};
use axum::{extract::State, Extension, Json};
use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct UsageResponse {
    pub plan: String,
    pub current_month: CurrentMonthUsage,
    /// Last 12 months excluding the current one, oldest first.
    pub history: Vec<UsageHistoryEntry>,
}

#[derive(Serialize)]
pub struct CurrentMonthUsage {
    pub month: NaiveDate,
    pub count: i32,
    pub quota: u32,
}

#[derive(Serialize)]
pub struct UsageHistoryEntry {
    pub month: NaiveDate,
    pub count: i32,
}

pub async fn get_usage(
    State(state): State<AppState>,
    Extension(customer_id): Extension<Uuid>,
) -> Result<Json<UsageResponse>, ApiError> {
    get_usage_impl(&state, customer_id).await
}

/// Pure body of `GET /v1/usage` and `GET /dashboard/usage`. Customer id
/// passed directly so either auth path can call it.
pub async fn get_usage_impl(
    state: &AppState,
    customer_id: Uuid,
) -> Result<Json<UsageResponse>, ApiError> {
    let plan_str = customer::get_plan(&state.db, customer_id)
        .await
        .map_err(internal_err)?
        .ok_or(ApiError::NotFound("customer"))?;

    let count = db::current_month_count(&state.db, customer_id)
        .await
        .map_err(internal_err)?;
    let history = db::usage_history(&state.db, customer_id)
        .await
        .map_err(internal_err)?;

    let today = chrono::Utc::now().date_naive();
    let current_month = today.with_day(1).unwrap_or(today);

    Ok(Json(UsageResponse {
        plan: plan_str.clone(),
        current_month: CurrentMonthUsage {
            month: current_month,
            count,
            quota: plan::quota_for(&plan_str),
        },
        history: history
            .into_iter()
            .map(|r| UsageHistoryEntry {
                month: r.month,
                count: r.proof_count,
            })
            .collect(),
    }))
}
