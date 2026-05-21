use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::api_key;

pub struct Customer {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
    pub plan: String,
}

/// Create a new customer and mint its first API key in a single transaction.
/// The plaintext key is returned exactly once, here.
pub async fn create(pool: &PgPool, name: &str) -> Result<(Customer, String), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let customer = sqlx::query_as!(
        Customer,
        r#"
        INSERT INTO customers (name)
        VALUES ($1)
        RETURNING id, name, created_at, is_active, plan
        "#,
        name,
    )
    .fetch_one(&mut *tx)
    .await?;
    let (_key, plaintext) = api_key::create(&mut tx, customer.id, "default").await?;
    tx.commit().await?;
    Ok((customer, plaintext))
}

pub async fn list(pool: &PgPool) -> Result<Vec<Customer>, sqlx::Error> {
    sqlx::query_as!(
        Customer,
        r#"
        SELECT id, name, created_at, is_active, plan
        FROM customers
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

/// Soft-delete the customer. Idempotent. Active API keys are left alone —
/// `require_customer_key` joins on `customers.is_active` so they stop
/// authenticating immediately anyway.
pub async fn deactivate(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE customers SET is_active = false WHERE id = $1"#,
        id,
    )
    .execute(pool)
    .await?;
    Ok(())
}
