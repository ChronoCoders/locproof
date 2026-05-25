//! Stripe webhook (`POST /stripe/webhook`). Authenticated by Stripe signature,
//! not API key or session — so it lives outside every auth router. Plan state
//! is driven off subscription lifecycle events; `checkout.session.completed`
//! is informational because the subscription events carry the price + status.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use stripe::{EventObject, EventType, Subscription, Webhook};

use crate::error::{internal_err, ApiError};
use crate::models::customer;
use crate::state::AppState;

pub async fn handle(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    let secret = state.stripe_webhook_secret.as_ref().ok_or_else(|| {
        tracing::error!("stripe webhook hit but STRIPE_WEBHOOK_SECRET is unset");
        ApiError::Internal
    })?;
    let signature = headers
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::BadRequest("missing Stripe-Signature".into()))?;
    let payload =
        std::str::from_utf8(&body).map_err(|_| ApiError::BadRequest("non-utf8 body".into()))?;

    let event = Webhook::construct_event(payload, signature, secret).map_err(|e| {
        tracing::warn!("stripe webhook signature verification failed: {e:?}");
        ApiError::BadRequest("invalid signature".into())
    })?;

    match event.type_ {
        EventType::CustomerSubscriptionCreated | EventType::CustomerSubscriptionUpdated => {
            if let EventObject::Subscription(sub) = event.data.object {
                sync_subscription(&state, &sub).await?;
            } else {
                tracing::warn!(
                    "{:?} carried non-subscription payload; ignoring",
                    event.type_
                );
            }
        }
        EventType::CustomerSubscriptionDeleted => {
            if let EventObject::Subscription(sub) = event.data.object {
                let stripe_customer = sub.customer.id();
                if let Some(id) =
                    customer::find_by_stripe_customer_id(&state.db, stripe_customer.as_str())
                        .await
                        .map_err(internal_err)?
                {
                    customer::set_plan(&state.db, id, "free", None, None)
                        .await
                        .map_err(internal_err)?;
                    tracing::info!("subscription canceled; reverted customer {id} to free");
                }
            } else {
                tracing::warn!("subscription.deleted carried non-subscription payload; ignoring");
            }
        }
        EventType::CheckoutSessionCompleted => {
            tracing::info!(
                "checkout.session.completed received; plan synced via subscription events"
            );
        }
        other => tracing::debug!("ignoring stripe event: {other:?}"),
    }

    Ok(StatusCode::OK)
}

/// Reconcile one customer's plan from a subscription's current price + status.
/// Entitled statuses keep the mapped paid plan; any other status downgrades to
/// free (cancellation also arrives as a separate `deleted` event). Unknown
/// customers or unmapped prices are acked and ignored to avoid Stripe retries.
async fn sync_subscription(state: &AppState, sub: &Subscription) -> Result<(), ApiError> {
    let stripe_customer = sub.customer.id();
    let Some(id) = customer::find_by_stripe_customer_id(&state.db, stripe_customer.as_str())
        .await
        .map_err(internal_err)?
    else {
        tracing::warn!("subscription for unknown stripe customer {stripe_customer}");
        return Ok(());
    };

    let Some(price_id) = sub
        .items
        .data
        .first()
        .and_then(|item| item.price.as_ref())
        .map(|price| price.id.as_str())
    else {
        tracing::warn!("subscription {} carries no price; skipping", sub.id);
        return Ok(());
    };

    let Some(plan) = state.stripe_prices.plan_for_price(price_id) else {
        tracing::warn!(
            "subscription {} has unmapped price {price_id}; skipping",
            sub.id
        );
        return Ok(());
    };

    let status = sub.status.as_str();
    let entitled = matches!(status, "active" | "trialing" | "past_due");
    let effective_plan = if entitled { plan } else { "free" };

    customer::set_plan(
        &state.db,
        id,
        effective_plan,
        Some(sub.id.as_str()),
        Some(status),
    )
    .await
    .map_err(internal_err)?;
    tracing::info!("synced customer {id} to plan {effective_plan} (subscription status {status})");
    Ok(())
}
