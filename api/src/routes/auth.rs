use crate::auth::SESSION_COOKIE;
use crate::error::{internal_err, ApiError};
use crate::models::{customer, session, user};
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use stripe::{CreateCustomer, Customer as StripeCustomer};
use uuid::Uuid;

const MIN_PASSWORD_LEN: usize = 12;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    /// Descriptive label for the customer record (org name, project name).
    pub customer_name: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub user_id: Uuid,
    pub customer_id: Uuid,
    pub email: String,
    pub session_expires_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user_id: Uuid,
    pub customer_id: Uuid,
    pub session_expires_at: DateTime<Utc>,
}

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, CookieJar, Json<RegisterResponse>), ApiError> {
    let email = req.email.trim().to_lowercase();
    validate_email(&email)?;
    validate_password(&req.password)?;
    let customer_name = req.customer_name.trim();
    if customer_name.is_empty() {
        return Err(ApiError::BadRequest(
            "customer_name must not be empty".into(),
        ));
    }

    let password_hash = user::hash_password(&req.password).map_err(internal_err)?;

    let stripe_customer_id = if let Some(client) = state.stripe.as_ref() {
        let created = StripeCustomer::create(
            client,
            CreateCustomer {
                email: Some(&email),
                ..Default::default()
            },
        )
        .await
        .map_err(internal_err)?;
        Some(created.id.to_string())
    } else {
        None
    };

    let mut tx = state.db.begin().await.map_err(internal_err)?;
    let cust = customer::create_for_user(&mut tx, customer_name, stripe_customer_id.as_deref())
        .await
        .map_err(internal_err)?;
    let new_user = user::create(&mut tx, &email, &password_hash, cust.id)
        .await
        .map_err(internal_err)?;
    tx.commit().await.map_err(internal_err)?;

    let (_session_id, expires_at, token) = session::create(&state.db, new_user.id)
        .await
        .map_err(internal_err)?;

    let jar = jar.add(build_session_cookie(token, state.cookie_secure));
    Ok((
        StatusCode::CREATED,
        jar,
        Json(RegisterResponse {
            user_id: new_user.id,
            customer_id: cust.id,
            email,
            session_expires_at: expires_at,
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<(CookieJar, Json<LoginResponse>), ApiError> {
    let email = req.email.trim().to_lowercase();

    let found = user::find_with_password_hash(&state.db, &email)
        .await
        .map_err(internal_err)?;
    let Some((found_user, password_hash)) = found else {
        return Err(ApiError::Unauthorized);
    };
    if !user::verify_password(&password_hash, &req.password) {
        return Err(ApiError::Unauthorized);
    }

    let (_session_id, expires_at, token) = session::create(&state.db, found_user.id)
        .await
        .map_err(internal_err)?;

    let jar = jar.add(build_session_cookie(token, state.cookie_secure));
    Ok((
        jar,
        Json(LoginResponse {
            user_id: found_user.id,
            customer_id: found_user.customer_id,
            session_expires_at: expires_at,
        }),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), ApiError> {
    if let Some(c) = jar.get(SESSION_COOKIE) {
        session::delete_by_token(&state.db, c.value())
            .await
            .map_err(internal_err)?;
    }
    let jar = jar.remove(Cookie::from(SESSION_COOKIE));
    Ok((jar, StatusCode::NO_CONTENT))
}

fn validate_email(email: &str) -> Result<(), ApiError> {
    // Intentionally minimal: dashboard email verification (4a out of scope)
    // will catch typos; we just reject obvious garbage here.
    if !email.contains('@') || email.len() < 3 || email.len() > 254 {
        return Err(ApiError::BadRequest("invalid email".into()));
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(ApiError::BadRequest(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }
    Ok(())
}

fn build_session_cookie(token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(time::Duration::days(session::SESSION_SLIDING_DAYS))
        .build()
}
