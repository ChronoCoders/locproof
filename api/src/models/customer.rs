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

/// Create a customer for a dashboard user registration. No initial API
/// key — the user mints one explicitly from `/dashboard/keys` later.
/// `stripe_customer_id` is `None` if Stripe wasn't configured at boot.
pub async fn create_for_user(
    conn: &mut sqlx::PgConnection,
    name: &str,
    stripe_customer_id: Option<&str>,
) -> Result<Customer, sqlx::Error> {
    sqlx::query_as!(
        Customer,
        r#"
        INSERT INTO customers (name, stripe_customer_id)
        VALUES ($1, $2)
        RETURNING id, name, created_at, is_active, plan
        "#,
        name,
        stripe_customer_id,
    )
    .fetch_one(&mut *conn)
    .await
}

/// Fetch the plan string for a single customer. `None` if no such id.
pub async fn get_plan(pool: &PgPool, id: Uuid) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT plan FROM customers WHERE id = $1"#, id)
        .fetch_optional(pool)
        .await
}

/// Resolve our customer id from a Stripe customer id. `None` if no customer
/// is linked to it — the webhook treats that as "ack and ignore".
pub async fn find_by_stripe_customer_id(
    pool: &PgPool,
    stripe_customer_id: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT id FROM customers WHERE stripe_customer_id = $1"#,
        stripe_customer_id,
    )
    .fetch_optional(pool)
    .await
}

/// Sync a customer's plan and Stripe subscription linkage. Called from the
/// Stripe webhook on subscription create/change/cancel. Passing `None` for the
/// subscription fields clears them — e.g. on cancellation, where the caller
/// also passes `plan = "free"`. Returns `false` if no customer matched the id.
pub async fn set_plan(
    pool: &PgPool,
    id: Uuid,
    plan: &str,
    stripe_subscription_id: Option<&str>,
    subscription_status: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE customers
        SET plan = $2, stripe_subscription_id = $3, subscription_status = $4
        WHERE id = $1
        "#,
        id,
        plan,
        stripe_subscription_id,
        subscription_status,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
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
