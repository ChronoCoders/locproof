//! Plan-sync persistence tests (Phase 4c). Drives `customer::set_plan`
//! against a real Postgres. Skipped unless `DATABASE_URL` is set.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ed25519_dalek::SigningKey;
use locproof_api::models::customer;
use locproof_api::state::{AppState, StripeConfig};
use locproof_api::{build_app, db, ratelimit};
use rand::rngs::OsRng;
use tower::util::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn set_plan_syncs_and_clears_subscription() {
    let Ok(db_url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL not set; skipping integration test");
        return;
    };

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&db_url)
        .await
        .expect("connect db");
    db::run_migrations(&pool).await.expect("migrate");

    let unique = format!("e2e-billing-{}", Uuid::new_v4());
    let (cust, _key) = customer::create(&pool, &unique)
        .await
        .expect("create customer");
    assert_eq!(cust.plan, "free", "new customer defaults to free");

    let matched = customer::set_plan(&pool, cust.id, "growth", Some("sub_123"), Some("active"))
        .await
        .expect("set_plan upgrade");
    assert!(matched, "upgrade must match the customer row");

    let (plan, sub_id, status): (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT plan, stripe_subscription_id, subscription_status FROM customers WHERE id = $1",
    )
    .bind(cust.id)
    .fetch_one(&pool)
    .await
    .expect("read after upgrade");
    assert_eq!(plan, "growth");
    assert_eq!(sub_id.as_deref(), Some("sub_123"));
    assert_eq!(status.as_deref(), Some("active"));

    // Cancellation reverts to free and clears the subscription linkage.
    let matched = customer::set_plan(&pool, cust.id, "free", None, None)
        .await
        .expect("set_plan cancel");
    assert!(matched);

    let (plan, sub_id, status): (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT plan, stripe_subscription_id, subscription_status FROM customers WHERE id = $1",
    )
    .bind(cust.id)
    .fetch_one(&pool)
    .await
    .expect("read after cancel");
    assert_eq!(plan, "free");
    assert_eq!(sub_id, None, "subscription id cleared on cancel");
    assert_eq!(status, None, "subscription status cleared on cancel");

    let missing = customer::set_plan(&pool, Uuid::new_v4(), "growth", None, None)
        .await
        .expect("set_plan unknown id");
    assert!(!missing, "unknown customer id must not match");

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

#[tokio::test]
async fn webhook_rejects_missing_and_invalid_signature() {
    let Ok(db_url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL not set; skipping integration test");
        return;
    };

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&db_url)
        .await
        .expect("connect db");
    db::run_migrations(&pool).await.expect("migrate");

    let server_key = SigningKey::generate(&mut OsRng);
    let limiter = ratelimit::create_limiter(1000).expect("limiter");
    let stripe = StripeConfig {
        webhook_secret: Some("whsec_test".into()),
        ..Default::default()
    };
    let app = build_app(AppState::new(
        server_key, None, limiter, pool, stripe, false,
    ));

    let no_sig = Request::builder()
        .method("POST")
        .uri("/stripe/webhook")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app.clone().oneshot(no_sig).await.expect("oneshot no-sig");
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "missing Stripe-Signature must be 400"
    );

    let bad_sig = Request::builder()
        .method("POST")
        .uri("/stripe/webhook")
        .header("Stripe-Signature", "t=123,v1=deadbeef")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app.clone().oneshot(bad_sig).await.expect("oneshot bad-sig");
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "invalid signature must be 400"
    );
}
