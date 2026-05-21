use chrono::Datelike;
use locproof_core::proof::ProximityProof;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

/// Connect to Postgres using `DATABASE_URL` and verify the connection.
pub async fn create_pool() -> anyhow::Result<PgPool> {
    let url = std::env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL not set"))?;
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await?;
    Ok(pool)
}

/// Apply all migrations under `./migrations` to the connected database.
pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("../migrations").run(pool).await?;
    Ok(())
}

/// Persist a verified, server-signed proof. Takes `&mut PgConnection` so
/// the caller controls the transaction (the `POST /v1/proofs` handler
/// bundles this with `increment_usage` in a single tx).
pub async fn store_proof(
    conn: &mut PgConnection,
    customer_id: Uuid,
    proof: &ProximityProof,
) -> anyhow::Result<()> {
    let proof_data = serde_json::to_value(proof)?;
    sqlx::query!(
        r#"
        INSERT INTO proofs (
            id, customer_id, device_a_pubkey, device_b_pubkey,
            proximity_score, server_signature, proof_data
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        proof.id,
        customer_id,
        &proof.device_a.device_id[..],
        &proof.device_b.device_id[..],
        proof.proximity_score,
        &proof.server_signature[..],
        proof_data,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// One row of `GET /v1/proofs` — summary fields only. The full signed
/// proof is in `proof_data` JSONB; callers who want it call `get_proof`.
pub struct ProofSummaryRow {
    pub id: Uuid,
    pub proximity_score: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Cursor-paginated list of a customer's proofs, newest first. Order is
/// `(created_at DESC, id DESC)`; the cursor is the last row's
/// `(created_at, id)` tuple. `None` cursor returns the first page.
pub async fn list_proofs(
    pool: &PgPool,
    customer_id: Uuid,
    cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
    limit: i64,
) -> anyhow::Result<Vec<ProofSummaryRow>> {
    let (cursor_ts, cursor_id) = match cursor {
        Some((ts, id)) => (Some(ts), Some(id)),
        None => (None, None),
    };
    let rows = sqlx::query_as!(
        ProofSummaryRow,
        r#"
        SELECT id, proximity_score, created_at
        FROM proofs
        WHERE customer_id = $1
          AND (
            $2::TIMESTAMPTZ IS NULL
            OR (created_at, id) < ($2::TIMESTAMPTZ, $3::UUID)
          )
        ORDER BY created_at DESC, id DESC
        LIMIT $4
        "#,
        customer_id,
        cursor_ts,
        cursor_id,
        limit,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Current-month proof count for a customer. Returns 0 if no row exists
/// (the customer hasn't submitted anything this month yet).
pub async fn current_month_count(pool: &PgPool, customer_id: Uuid) -> anyhow::Result<i32> {
    let today = chrono::Utc::now().date_naive();
    let month = today.with_day(1).unwrap_or(today);
    let row = sqlx::query_scalar!(
        r#"SELECT proof_count FROM usage WHERE customer_id = $1 AND month = $2"#,
        customer_id,
        month,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.unwrap_or(0))
}

/// One row of `GET /v1/usage` history.
pub struct UsageHistoryRow {
    pub month: chrono::NaiveDate,
    pub proof_count: i32,
}

/// Last 12 months of usage for a customer, oldest first, excluding the
/// current month (the caller pairs this with `current_month_count`).
pub async fn usage_history(
    pool: &PgPool,
    customer_id: Uuid,
) -> anyhow::Result<Vec<UsageHistoryRow>> {
    let today = chrono::Utc::now().date_naive();
    let current_month = today.with_day(1).unwrap_or(today);
    let rows = sqlx::query_as!(
        UsageHistoryRow,
        r#"
        SELECT month, proof_count
        FROM usage
        WHERE customer_id = $1
          AND month < $2
          AND month >= $2 - INTERVAL '12 months'
        ORDER BY month ASC
        "#,
        customer_id,
        current_month,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Fetch a stored proof by id. `Ok(None)` means the row was not found —
/// callers translate that to a 404. Pool-bound (read-only, no transaction
/// coordination needed).
pub async fn get_proof(pool: &PgPool, proof_id: Uuid) -> anyhow::Result<Option<ProximityProof>> {
    let row = sqlx::query!(r#"SELECT proof_data FROM proofs WHERE id = $1"#, proof_id,)
        .fetch_optional(pool)
        .await?;

    match row {
        Some(r) => Ok(Some(serde_json::from_value(r.proof_data)?)),
        None => Ok(None),
    }
}

/// Bump the customer's current-month proof count. Takes `&mut PgConnection`
/// for the same reason as `store_proof`. `with_day(1)` is infallible for a
/// valid `NaiveDate`; `unwrap_or(today)` is a no-panic guard, not a real
/// fallback.
pub async fn increment_usage(conn: &mut PgConnection, customer_id: Uuid) -> anyhow::Result<()> {
    let today = chrono::Utc::now().date_naive();
    let month = today.with_day(1).unwrap_or(today);
    sqlx::query!(
        r#"
        INSERT INTO usage (customer_id, month, proof_count)
        VALUES ($1, $2, 1)
        ON CONFLICT (customer_id, month)
        DO UPDATE SET proof_count = usage.proof_count + 1
        "#,
        customer_id,
        month,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}
