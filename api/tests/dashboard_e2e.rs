//! End-to-end test for /auth/register → /dashboard/keys → /v1/* → logout.
//! Covers session cookie round-trip, customer-scoped key creation, and the
//! revocation path. Skipped unless `DATABASE_URL` is set.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use http_body_util::BodyExt;
use locproof_api::ratelimit;
use locproof_api::state::AppState;
use locproof_api::{build_app, db};
use locproof_core::signals::SignalSnapshot;
use rand::rngs::OsRng;
use serde::Serialize;
use serde_json::{json, Value};
use tower::util::ServiceExt;
use uuid::Uuid;

#[derive(Serialize)]
struct WireAtt {
    device_id: String,
    signals: SignalSnapshot,
    signature: String,
}

#[derive(Serialize)]
struct SubmitReq {
    device_a: WireAtt,
    device_b: WireAtt,
}

fn signed_att(key: &SigningKey, ble_id: &str, ts: i64) -> WireAtt {
    let snapshot = SignalSnapshot {
        device_id: ble_id.to_string(),
        collected_at: ts,
        gps: None,
        ble_devices: vec![],
        barometer: None,
    };
    let msg = bincode::serialize(&snapshot).expect("serialize");
    let sig = key.sign(&msg);
    WireAtt {
        device_id: STANDARD.encode(key.verifying_key().to_bytes()),
        signals: snapshot,
        signature: STANDARD.encode(sig.to_bytes()),
    }
}

/// Extract the value of `Set-Cookie: lp_session=...` (just the token, no
/// attributes). Returns `None` if the cookie isn't present.
fn extract_session_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .find_map(|s| {
            let prefix = "lp_session=";
            let after = s.strip_prefix(prefix)?;
            let value = after.split(';').next()?;
            Some(value.to_string())
        })
}

#[tokio::test]
async fn register_mint_key_use_revoke_logout() {
    let Ok(db_url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL not set; skipping integration test");
        return;
    };

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&db_url)
        .await
        .expect("connect db");
    db::run_migrations(&pool).await.expect("migrate");

    let server_key = SigningKey::generate(&mut OsRng);
    let limiter = ratelimit::create_limiter(1000).expect("limiter");
    let state = AppState::new(server_key, None, limiter, pool.clone(), None, false);
    let app = build_app(state);

    // Register a fresh user.
    let unique = format!("e2e-{}", Uuid::new_v4());
    let email = format!("{unique}@example.com");
    let register_body = json!({
        "email": email,
        "password": "this-is-a-long-test-password",
        "customer_name": unique,
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&register_body).unwrap()))
                .unwrap(),
        )
        .await
        .expect("oneshot register");
    assert_eq!(resp.status(), StatusCode::CREATED, "register status");
    let session_cookie = extract_session_cookie(resp.headers()).expect("session cookie set");
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let customer_id: Uuid = body["customer_id"].as_str().unwrap().parse().unwrap();
    let user_id: Uuid = body["user_id"].as_str().unwrap().parse().unwrap();

    let cookie_header = format!("lp_session={session_cookie}");

    // Mint a key via the dashboard.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/dashboard/keys")
                .header("Content-Type", "application/json")
                .header("Cookie", &cookie_header)
                .body(Body::from(
                    serde_json::to_vec(&json!({"name": "test-key"})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .expect("oneshot create_key");
    assert_eq!(resp.status(), StatusCode::CREATED, "create_key status");
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let api_key = body["api_key"].as_str().unwrap().to_string();
    let key_id: Uuid = body["id"].as_str().unwrap().parse().unwrap();

    // Listing returns it.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/keys")
                .header("Cookie", &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot list_keys");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let list: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(list
        .as_array()
        .unwrap()
        .iter()
        .any(|k| k["id"] == body["id"]));

    // The fresh key authenticates against /v1/proofs.
    let key_a = SigningKey::generate(&mut OsRng);
    let key_b = SigningKey::generate(&mut OsRng);
    let ts = chrono::Utc::now().timestamp();
    let proof_body = SubmitReq {
        device_a: signed_att(&key_a, "ble-a", ts),
        device_b: signed_att(&key_b, "ble-b", ts + 1),
    };
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/proofs")
                .header("Content-Type", "application/json")
                .header("X-API-Key", &api_key)
                .body(Body::from(serde_json::to_vec(&proof_body).unwrap()))
                .unwrap(),
        )
        .await
        .expect("oneshot proof submit");
    assert_eq!(resp.status(), StatusCode::OK, "submit with fresh key");

    // Delete the key.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/dashboard/keys/{key_id}"))
                .header("Cookie", &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot delete_key");
    assert_eq!(resp.status(), StatusCode::OK);

    // Submitting again with the same (now deactivated) key fails.
    let proof_body = SubmitReq {
        device_a: signed_att(&key_a, "ble-a", ts + 10),
        device_b: signed_att(&key_b, "ble-b", ts + 11),
    };
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/proofs")
                .header("Content-Type", "application/json")
                .header("X-API-Key", &api_key)
                .body(Body::from(serde_json::to_vec(&proof_body).unwrap()))
                .unwrap(),
        )
        .await
        .expect("oneshot proof submit after revoke");
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "revoked key must not authenticate"
    );

    // Cross-tenant probe: trying to delete a random uuid returns 404.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/dashboard/keys/{}", Uuid::new_v4()))
                .header("Cookie", &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot delete unknown");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Logout deletes the session.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/logout")
                .header("Cookie", &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot logout");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Dashboard request with the now-stale cookie is unauthorised.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/keys")
                .header("Cookie", &cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot after logout");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Cleanup. Order: proofs / usage / sessions / api_keys / users / customer.
    sqlx::query("DELETE FROM proofs WHERE customer_id = $1")
        .bind(customer_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM usage WHERE customer_id = $1")
        .bind(customer_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM api_keys WHERE customer_id = $1")
        .bind(customer_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE customer_id = $1")
        .bind(customer_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM customers WHERE id = $1")
        .bind(customer_id)
        .execute(&pool)
        .await
        .unwrap();
}
