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
