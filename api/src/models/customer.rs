use chrono::{DateTime, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub struct Customer {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

/// Generate a customer API key in the form `lp_live_<32 hex chars>` from
/// 16 bytes of OS randomness.
pub fn generate_api_key() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("lp_live_{}", hex::encode(bytes))
}

/// SHA-256 of an API key, hex-encoded. This is what we persist and what
/// `require_customer_key` looks up against `customers.api_key_hash`.
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Insert a new customer with a freshly generated key. Returns the persisted
/// row and the plaintext key — the plaintext is only ever returned here.
pub async fn create(pool: &PgPool, name: &str) -> Result<(Customer, String), sqlx::Error> {
    let api_key = generate_api_key();
    let hash = hash_api_key(&api_key);
    let row = sqlx::query_as!(
        Customer,
        r#"
        INSERT INTO customers (name, api_key_hash)
        VALUES ($1, $2)
        RETURNING id, name, created_at, is_active
        "#,
        name,
        hash,
    )
    .fetch_one(pool)
    .await?;
    Ok((row, api_key))
}

pub async fn list(pool: &PgPool) -> Result<Vec<Customer>, sqlx::Error> {
    sqlx::query_as!(
        Customer,
        r#"
        SELECT id, name, created_at, is_active
        FROM customers
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

/// Soft-delete by flipping `is_active`. Idempotent: succeeds whether the id
/// matches an active row, an already-inactive row, or nothing at all.
pub async fn deactivate(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE customers SET is_active = false WHERE id = $1"#,
        id,
    )
    .execute(pool)
    .await?;
    Ok(())
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
