use chrono::{DateTime, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub struct ApiKey {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Generate an API key in the form `lp_live_<32 hex chars>` from 16 bytes
/// of OS randomness.
pub fn generate_api_key() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("lp_live_{}", hex::encode(bytes))
}

/// SHA-256 of an API key, hex-encoded. This is the value stored in
/// `api_keys.key_hash` and looked up by `require_customer_key`.
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Mint a new key on `customer_id`. Takes `&mut PgConnection` so the caller
/// controls the transaction (the customer-create flow bundles this with the
/// customer insert in one tx). Returns the persisted row and the plaintext
/// key — the plaintext is only ever returned here.
pub async fn create(
    conn: &mut PgConnection,
    customer_id: Uuid,
    name: &str,
) -> Result<(ApiKey, String), sqlx::Error> {
    let plaintext = generate_api_key();
    let hash = hash_api_key(&plaintext);
    let row = sqlx::query_as!(
        ApiKey,
        r#"
        INSERT INTO api_keys (customer_id, key_hash, name)
        VALUES ($1, $2, $3)
        RETURNING id, customer_id, name, created_at, last_used_at, is_active
        "#,
        customer_id,
        hash,
        name,
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok((row, plaintext))
}

pub async fn list_for_customer(
    pool: &PgPool,
    customer_id: Uuid,
) -> Result<Vec<ApiKey>, sqlx::Error> {
    sqlx::query_as!(
        ApiKey,
        r#"
        SELECT id, customer_id, name, created_at, last_used_at, is_active
        FROM api_keys
        WHERE customer_id = $1
        ORDER BY created_at DESC
        "#,
        customer_id,
    )
    .fetch_all(pool)
    .await
}

/// Soft-delete: mark the key inactive. Idempotent.
pub async fn deactivate(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(r#"UPDATE api_keys SET is_active = false WHERE id = $1"#, id,)
        .execute(pool)
        .await?;
    Ok(())
}

/// Soft-delete a key, scoped to a customer. Returns `true` iff a row
/// matched (key exists AND belongs to `customer_id`). The dashboard handler
/// maps `false` to 404 so cross-tenant probing returns the same response
/// as a non-existent id.
pub async fn deactivate_for_customer(
    pool: &PgPool,
    id: Uuid,
    customer_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"UPDATE api_keys SET is_active = false WHERE id = $1 AND customer_id = $2"#,
        id,
        customer_id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_have_expected_shape() {
        let k = generate_api_key();
        assert!(k.starts_with("lp_live_"));
        assert_eq!(k.len(), "lp_live_".len() + 32);
        assert!(k["lp_live_".len()..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generated_keys_are_unique() {
        assert_ne!(generate_api_key(), generate_api_key());
    }

    #[test]
    fn hash_is_deterministic() {
        let h1 = hash_api_key("lp_live_abc");
        let h2 = hash_api_key("lp_live_abc");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn different_keys_hash_differently() {
        assert_ne!(hash_api_key("a"), hash_api_key("b"));
    }
}
