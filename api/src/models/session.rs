use chrono::{DateTime, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Subset returned by `find_active_and_touch`; carries the user's
/// `customer_id` so the middleware can inject it without a second lookup.
pub struct ActiveSession {
    pub user_id: Uuid,
    pub customer_id: Uuid,
}

/// Sliding session length: each successful lookup advances `expires_at` by
/// this much, capped at [`SESSION_ABSOLUTE_MAX_DAYS`] from `created_at`.
pub const SESSION_SLIDING_DAYS: i64 = 30;
pub const SESSION_ABSOLUTE_MAX_DAYS: i64 = 90;

/// Generate a 32-byte URL-safe session token (256 bits of entropy).
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64_urlsafe(&bytes)
}

fn base64_urlsafe(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Mint a session for `user_id`. Initial `expires_at` is
/// `NOW() + SESSION_SLIDING_DAYS`. Returns the persisted id/expiry along
/// with the plaintext token (only returned here).
pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(Uuid, DateTime<Utc>, String), sqlx::Error> {
    let token = generate_token();
    let hash = hash_token(&token);
    let row = sqlx::query!(
        r#"
        INSERT INTO sessions (user_id, token_hash, expires_at)
        VALUES ($1, $2, NOW() + ($3 || ' days')::INTERVAL)
        RETURNING id, expires_at
        "#,
        user_id,
        hash,
        SESSION_SLIDING_DAYS.to_string(),
    )
    .fetch_one(pool)
    .await?;
    Ok((row.id, row.expires_at, token))
}

/// Look up an active session by plaintext token and slide its expiry in
/// a single UPDATE...FROM. Returns `None` if no row matches the token,
/// the session has already expired, or it has passed the absolute 90-day
/// cap from `created_at`.
pub async fn find_active_and_touch(
    pool: &PgPool,
    token: &str,
) -> Result<Option<ActiveSession>, sqlx::Error> {
    let hash = hash_token(token);
    let row = sqlx::query!(
        r#"
        UPDATE sessions s
        SET expires_at = LEAST(
                NOW() + ($2 || ' days')::INTERVAL,
                s.created_at + ($3 || ' days')::INTERVAL
            ),
            last_used_at = NOW()
        FROM users u
        WHERE s.token_hash = $1
          AND s.expires_at > NOW()
          AND s.created_at + ($3 || ' days')::INTERVAL > NOW()
          AND u.id = s.user_id
        RETURNING s.user_id, u.customer_id
        "#,
        hash,
        SESSION_SLIDING_DAYS.to_string(),
        SESSION_ABSOLUTE_MAX_DAYS.to_string(),
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| ActiveSession {
        user_id: r.user_id,
        customer_id: r.customer_id,
    }))
}

/// Logout: delete the session by token. Idempotent.
pub async fn delete_by_token(pool: &PgPool, token: &str) -> Result<(), sqlx::Error> {
    let hash = hash_token(token);
    sqlx::query!(r#"DELETE FROM sessions WHERE token_hash = $1"#, hash)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_are_unique_and_long() {
        let a = generate_token();
        let b = generate_token();
        assert_ne!(a, b);
        // 32 bytes base64url no-pad → 43 chars
        assert_eq!(a.len(), 43);
    }

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert_ne!(hash_token("abc"), hash_token("abd"));
    }
}
