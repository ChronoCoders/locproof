//! End-to-end test for `POST /v1/proofs` + `GET /v1/proofs/:id`. Drives the
//! real router (via `tower::ServiceExt::oneshot`) against a real Postgres.
//!
//! Skipped unless `DATABASE_URL` is set. The test creates its own customer,
//! exercises the full submit/retrieve/usage path, then deletes every row it
//! produced.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use http_body_util::BodyExt;
use locproof_api::models::customer;
use locproof_api::ratelimit;
use locproof_api::state::AppState;
use locproof_api::{build_app, db};
use locproof_core::proof::ProximityProof;
use locproof_core::signals::SignalSnapshot;
use rand::rngs::OsRng;
use serde::Serialize;
use serde_json::Value;
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
    let msg = bincode::serialize(&snapshot).expect("serialize snapshot");
    let sig = key.sign(&msg);
    WireAtt {
        device_id: STANDARD.encode(key.verifying_key().to_bytes()),
        signals: snapshot,
        signature: STANDARD.encode(sig.to_bytes()),
    }
}

#[tokio::test]
async fn submit_then_retrieve_records_usage() {
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

    let unique = format!("e2e-{}", Uuid::new_v4());
    let (cust, api_key) = customer::create(&pool, &unique)
        .await
        .expect("create customer");

    let server_key = SigningKey::generate(&mut OsRng);
    let limiter = ratelimit::create_limiter(1000).expect("limiter");
    let state = AppState::new(server_key, None, limiter, pool.clone(), None, false);
    let app = build_app(state);

    let key_a = SigningKey::generate(&mut OsRng);
    let key_b = SigningKey::generate(&mut OsRng);
    let ts = chrono::Utc::now().timestamp();
    let req_body = SubmitReq {
        device_a: signed_att(&key_a, "ble-a", ts),
        device_b: signed_att(&key_b, "ble-b", ts + 1),
    };

    let post = Request::builder()
        .method("POST")
        .uri("/v1/proofs")
        .header("Content-Type", "application/json")
        .header("X-API-Key", &api_key)
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(post).await.expect("oneshot POST");
    assert_eq!(resp.status(), StatusCode::OK, "POST status");
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let proof_id: Uuid = body["proof_id"].as_str().unwrap().parse().unwrap();
    assert_eq!(body["verified"], true);

    let get = Request::builder()
        .method("GET")
        .uri(format!("/v1/proofs/{proof_id}"))
        .header("X-API-Key", &api_key)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(get).await.expect("oneshot GET");
    assert_eq!(resp.status(), StatusCode::OK, "GET status");
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let fetched: ProximityProof = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(fetched.id, proof_id);
    assert_eq!(fetched.device_a.device_id, key_a.verifying_key().to_bytes());

    let missing = Request::builder()
        .method("GET")
        .uri(format!("/v1/proofs/{}", Uuid::new_v4()))
        .header("X-API-Key", &api_key)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(missing).await.expect("oneshot 404");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "missing proof status");

    // IDOR guard: a *different* customer must not be able to retrieve this
    // proof by id. `get_proof` is scoped by customer_id, so the row is
    // invisible to other customers and the lookup 404s.
    let other_unique = format!("e2e-other-{}", Uuid::new_v4());
    let (other_cust, other_key) = customer::create(&pool, &other_unique)
        .await
        .expect("create other customer");
    let cross = Request::builder()
        .method("GET")
        .uri(format!("/v1/proofs/{proof_id}"))
        .header("X-API-Key", &other_key)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(cross).await.expect("oneshot cross");
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "cross-customer proof must 404"
    );
    sqlx::query("DELETE FROM api_keys WHERE customer_id = $1")
        .bind(other_cust.id)
        .execute(&pool)
        .await
        .expect("cleanup other api_keys");
    sqlx::query("DELETE FROM customers WHERE id = $1")
        .bind(other_cust.id)
        .execute(&pool)
        .await
        .expect("cleanup other customer");

    let count: Option<i32> =
        sqlx::query_scalar("SELECT proof_count FROM usage WHERE customer_id = $1")
            .bind(cust.id)
            .fetch_optional(&pool)
            .await
            .expect("fetch usage");
    assert_eq!(count, Some(1), "usage counter");

    // Cleanup: proofs / usage / api_keys all FK customer, drop them first.
    sqlx::query("DELETE FROM proofs WHERE customer_id = $1")
        .bind(cust.id)
        .execute(&pool)
        .await
        .expect("cleanup proofs");
    sqlx::query("DELETE FROM usage WHERE customer_id = $1")
        .bind(cust.id)
        .execute(&pool)
        .await
        .expect("cleanup usage");
    sqlx::query("DELETE FROM api_keys WHERE customer_id = $1")
        .bind(cust.id)
        .execute(&pool)
        .await
        .expect("cleanup api_keys");
    sqlx::query("DELETE FROM customers WHERE id = $1")
        .bind(cust.id)
        .execute(&pool)
        .await
        .expect("cleanup customer");
}
