use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub struct User {
    pub id: Uuid,
    pub email: String,
    pub customer_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Hash a password with argon2id (current OWASP-recommended algorithm and
/// crate defaults). The returned string is the full PHC-format hash, safe
/// to store as-is.
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Constant-time verify. Returns `false` on any error (malformed hash
/// included) so callers can treat the result as a single boolean — they
/// shouldn't leak whether the hash was malformed vs the password wrong.
pub fn verify_password(stored_hash: &str, password: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Insert a user. `password_hash` must already be argon2-hashed; this
/// function does not re-hash.
pub async fn create(
    conn: &mut PgConnection,
    email: &str,
    password_hash: &str,
    customer_id: Uuid,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (email, password_hash, customer_id)
        VALUES ($1, $2, $3)
        RETURNING id, email, customer_id, created_at
        "#,
        email,
        password_hash,
        customer_id,
    )
    .fetch_one(&mut *conn)
    .await
}

/// Look up a user by email and return them alongside their stored password
/// hash so the caller can run `verify_password`. Returns `Ok(None)` if no
/// such email exists.
pub async fn find_with_password_hash(
    pool: &PgPool,
    email: &str,
) -> Result<Option<(User, String)>, sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT id, email, password_hash, customer_id, created_at FROM users WHERE email = $1"#,
        email,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| {
        (
            User {
                id: r.id,
                email: r.email,
                customer_id: r.customer_id,
                created_at: r.created_at,
            },
            r.password_hash,
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrip() {
        let h = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password(&h, "correct horse battery staple"));
        assert!(!verify_password(&h, "wrong password"));
    }

    #[test]
    fn malformed_hash_returns_false() {
        assert!(!verify_password("not-a-real-hash", "anything"));
    }

    #[test]
    fn salt_makes_two_hashes_different() {
        let h1 = hash_password("same input").unwrap();
        let h2 = hash_password("same input").unwrap();
        assert_ne!(h1, h2);
    }
}
