use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use db::PgPool;
use mandate_engine::purchase_auth::PurchaseAuth;
use mandate_engine::{Frequency, Mandate, MandateSigner, NewMandate};
use policy_engine::PolicyEvaluator;
use razorpay_client::RazorpayClient;
use reconciliation::{RazorpayRefundProvider, ReconcileService};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod manifest;
mod auth;
use manifest::{Availability, MerchantManifest, Product};

// ─── App State ────────────────────────────────────────────────────────────

#[derive(Clone)]
#[allow(clippy::type_complexity)]
struct AppState {
    db: PgPool,
    signer: Arc<MandateSigner>,
    razorpay: Arc<RazorpayClient>,
    catalog: db::catalog_repo::CatalogRepo,
    merchant: db::merchant_repo::MerchantRepo,
    merchant_id: String,
    jwt_secret: Vec<u8>,
    reconcile: ReconcileService<RazorpayRefundProvider>,
    // In-process cache for hot read paths (catalog/manifest) — TTL bounded.
    catalog_cache: Arc<Mutex<Option<(Instant, Vec<db::catalog_repo::CatalogProductRow>)>>>,
    // Fixed-window rate limiter for money-moving endpoints, keyed by API key id.
    rate_limiter: Arc<Mutex<HashMap<String, (Instant, u64)>>>,
}

/// Cache TTL for catalog/manifest reads.
const CACHE_TTL: Duration = Duration::from_secs(10);
/// Money-endpoint rate limit: requests per fixed window per API key.
const MONEY_WINDOW: Duration = Duration::from_secs(60);
const MONEY_MAX_PER_WINDOW: u64 = 30;

/// Returns `429` if the key has exceeded its money-endpoint budget this window.
#[allow(clippy::type_complexity)]
fn check_rate_limit(
    limiter: &Arc<Mutex<HashMap<String, (Instant, u64)>>>,
    key_id: &Uuid,
) -> Result<(), StatusCode> {
    let mut map = limiter.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let now = Instant::now();
    match map.get_mut(&key_id.to_string()) {
        Some((start, count)) if now.duration_since(*start) < MONEY_WINDOW => {
            if *count >= MONEY_MAX_PER_WINDOW {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
            *count += 1;
        }
        other => {
            let entry = (now, 1u64);
            match other {
                Some(slot) => *slot = entry,
                None => {
                    map.insert(key_id.to_string(), entry);
                }
            }
        }
    }
    Ok(())
}

/// Returns cached catalog if fresh, else repopulates it from the repo.
async fn get_cached_catalog(
    state: &AppState,
) -> Result<Vec<db::catalog_repo::CatalogProductRow>, db::DbError> {
    {
        let guard = state.catalog_cache.lock().expect("catalog_cache poisoned");
        if let Some((ts, rows)) = guard.as_ref()
            && ts.elapsed() < CACHE_TTL
        {
            return Ok(rows.clone());
        }
    }
    let rows = state.catalog.list(&state.merchant_id).await?;
    let mut guard = state.catalog_cache.lock().expect("catalog_cache poisoned");
    *guard = Some((Instant::now(), rows.clone()));
    Ok(rows)
}

// ─── Request / Response Types ─────────────────────────────────────────────

#[derive(Deserialize)]
struct IssueMandateBody {
    merchant_id: String,
    max_amount: i64,
    currency: String,
    scope: Vec<String>,
    frequency: String,
    expires_in_hours: Option<i64>,
}

#[derive(Serialize)]
struct IssueMandateResponse {
    mandate_id: Uuid,
    signature: String,
    expires_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct PurchaseRequest {
    mandate_id: Uuid,
    product_id: String,
    quantity: i64,
    shipping_address: String,
    /// The price the buyer agent saw when it selected the product (paise).
    /// Reconciliation charges the live catalog price and compares it against
    /// this expected price to detect drift.
    expected_price: i64,
    /// Short explanation of why the agent selected this product (LLM
    /// reasoning). Optional; recorded on the intent for observability.
    reasoning: Option<String>,
    /// How the agent chose: "llm" or "rule-based". Optional.
    selection_method: Option<String>,
}

#[derive(Serialize)]
struct PurchaseResponse {
    order_id: String,
    payment_id: Option<String>,
    status: String,
    amount: i64,
}

#[derive(Serialize)]
struct AuditEntryResponse {
    event_type: String,
    entity_id: String,
    decision: String,
    reason: Option<String>,
    detail: Option<serde_json::Value>,
    actor: String,
    created_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
struct AuditTrailResponse {
    mandate_id: Uuid,
    entries: Vec<AuditEntryResponse>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
struct RevokeMandateBody {
    mandate_id: Uuid,
    reason: Option<String>,
}

#[derive(Serialize)]
struct RevokeMandateResponse {
    mandate_id: Uuid,
    status: String,
}

#[derive(Deserialize)]
struct SimulateDriftBody {
    product_id: String,
    new_price: i64,
}

#[derive(Serialize)]
struct SimulateDriftResponse {
    product_id: String,
    old_price: Option<i64>,
    new_price: i64,
    message: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────────

async fn get_manifest(State(state): State<AppState>) -> Json<MerchantManifest> {
    let m = state
        .merchant
        .get(&state.merchant_id)
        .await
        .ok()
        .flatten()
        .map(|row| MerchantManifest {
            merchant_id: row.merchant_id,
            merchant_name: row.name,
            capability_version: row.capability_version,
            supported_scopes: row.supported_scopes,
            catalog_endpoint: row.catalog_endpoint,
            mandate_endpoint: row.mandate_endpoint,
        })
        .unwrap_or_else(sample_manifest);
    Json(m)
}

async fn get_catalog_cached(State(state): State<AppState>) -> Result<Json<Vec<Product>>, (StatusCode, Json<ErrorResponse>)> {
    let rows = get_cached_catalog(&state).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
    Ok(Json(rows.into_iter().map(row_to_product).collect()))
}

#[derive(Serialize)]
struct MandateDetailResponse {
    mandate_id: Uuid,
    user_id: String,
    merchant_id: String,
    buyer_agent_id: String,
    max_amount: i64,
    currency: String,
    scope: Vec<String>,
    frequency: String,
    spent_amount: i64,
    status: String,
    nonce: String,
    signature: String,
    expires_at: chrono::DateTime<Utc>,
}

async fn get_mandate(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<MandateDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mandate = db::mandate_repo::find_by_id(&state.db, id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "mandate not found".into() })))?;

    Ok(Json(to_detail(&mandate)))
}

/// Maps a stored `Mandate` into the API response DTO (shared by the detail
/// and list endpoints).
fn to_detail(m: &Mandate) -> MandateDetailResponse {
    let status_str = match m.status {
        mandate_engine::MandateStatus::Active => "active",
        mandate_engine::MandateStatus::Revoked => "revoked",
        mandate_engine::MandateStatus::Expired => "expired",
        mandate_engine::MandateStatus::Exhausted => "exhausted",
    };
    let frequency_str = match m.frequency {
        mandate_engine::Frequency::OneTime => "one_time",
        mandate_engine::Frequency::Recurring => "recurring",
    };
    MandateDetailResponse {
        mandate_id: m.mandate_id,
        user_id: m.user_id.clone(),
        merchant_id: m.merchant_id.clone(),
        buyer_agent_id: m.buyer_agent_id.clone(),
        max_amount: m.max_amount,
        currency: m.currency.clone(),
        scope: m.scope.clone(),
        frequency: frequency_str.to_string(),
        spent_amount: m.spent_amount,
        status: status_str.to_string(),
        nonce: m.nonce.clone(),
        signature: hex::encode(&m.signature),
        expires_at: m.expires_at,
    }
}

#[derive(Deserialize)]
struct ListMandatesQuery {
    merchant_id: Option<String>,
}

#[derive(Deserialize)]
struct UpdatePriceBody {
    price: i64,
}

#[derive(Serialize)]
struct PriceUpdateResponse {
    product_id: String,
    price: i64,
}

/// Merchant-scoped list of mandates (operator console).
async fn list_mandates(
    auth::AuthUser(claims): auth::AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListMandatesQuery>,
) -> Result<Json<Vec<MandateDetailResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let merchant_id = params.merchant_id.or(claims.tenant_id).ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "merchant_id required".into(),
        }),
    ))?;

    let mandates = db::mandate_repo::list_by_merchant(&state.db, &merchant_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    let items = mandates.iter().map(to_detail).collect();
    Ok(Json(items))
}

/// Merchant edits a product's price (operator console).
async fn update_catalog_price(
    auth::AuthUser(claims): auth::AuthUser,
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    Json(body): Json<UpdatePriceBody>,
) -> Result<Json<PriceUpdateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let merchant_id = claims
        .tenant_id
        .ok_or((StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "merchant tenant required".into() })))?;

    let updated = state
        .catalog
        .set_price(&merchant_id, &product_id, body.price)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?
        .ok_or((StatusCode::NOT_FOUND, Json(ErrorResponse { error: "product not found".into() })))?;

    Ok(Json(PriceUpdateResponse {
        product_id: updated.product_id,
        price: updated.price,
    }))
}

/// Simulates catalog price drift for the engineered failure scenario.
///
/// Bumps a product's price so a subsequent purchase charges a different
/// amount than what the buyer agent intended, triggering reconciliation.
async fn simulate_drift(
    State(state): State<AppState>,
    Json(body): Json<SimulateDriftBody>,
) -> Result<Json<SimulateDriftResponse>, (StatusCode, Json<ErrorResponse>)> {
    let old_price = state
        .catalog
        .get(&state.merchant_id, &body.product_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?
        .map(|p| p.price);
    let updated = state
        .catalog
        .set_price(&state.merchant_id, &body.product_id, body.new_price)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "product not found".into() })))?;

    Ok(Json(SimulateDriftResponse {
        product_id: updated.product_id.clone(),
        old_price,
        new_price: updated.price,
        message: format!(
            "catalog price for {} drifted from {:?} to {}",
            updated.product_id, old_price, updated.price
        ),
    }))
}

async fn issue_mandate(
    auth::ApiKey(key): auth::ApiKey,
    State(state): State<AppState>,
    Json(body): Json<IssueMandateBody>,
) -> Result<(StatusCode, Json<IssueMandateResponse>), (StatusCode, Json<ErrorResponse>)> {
    if !has_scope(&key, "mandate:issue") {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse { error: "key missing scope: mandate:issue".into() })));
    }
    if check_rate_limit(&state.rate_limiter, &key.key_id).is_err() {
        return Err((StatusCode::TOO_MANY_REQUESTS, Json(ErrorResponse { error: "money endpoint rate limit exceeded".into() })));
    }
    let buyer_agent_id = key.tenant_id.clone().ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "agent key has no tenant_id".into() }))
    })?;

    let frequency = match body.frequency.as_str() {
        "recurring" => Frequency::Recurring,
        _ => Frequency::OneTime,
    };

    let expires_in_hours = body.expires_in_hours.unwrap_or(1);

    let params = NewMandate {
        user_id: key.account_id.to_string(),
        merchant_id: body.merchant_id,
        buyer_agent_id,
        max_amount: body.max_amount,
        currency: body.currency,
        scope: body.scope,
        frequency,
        expires_at: Utc::now() + chrono::Duration::hours(expires_in_hours),
    };

    let mut mandate = mandate_engine::Mandate::new(params);

    state.signer.sign(&mut mandate)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    // Persist
    db::mandate_repo::insert(&state.db, &mandate).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    // Audit: mandate issued
    db::audit_repo::append(&state.db, &db::audit_repo::AuditParams {
        event_type: "mandate_issued",
        mandate_id: Some(mandate.mandate_id),
        entity_id: &mandate.mandate_id.to_string(),
        decision: "allow",
        reason: None,
        detail: Some(serde_json::json!({
            "max_amount": mandate.max_amount,
            "scope": mandate.scope,
            "frequency": mandate.frequency,
        })),
        actor: &mandate.buyer_agent_id,
    }).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok((
        StatusCode::CREATED,
        Json(IssueMandateResponse {
            mandate_id: mandate.mandate_id,
            signature: hex::encode(&mandate.signature),
            expires_at: mandate.expires_at,
        }),
    ))
}

#[axum::debug_handler]
async fn execute_purchase(
    auth::ApiKey(key): auth::ApiKey,
    State(state): State<AppState>,
    Json(body): Json<PurchaseRequest>,
) -> Result<(StatusCode, Json<PurchaseResponse>), (StatusCode, Json<ErrorResponse>)> {
    if !has_scope(&key, "purchase:exec") {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse { error: "key missing scope: purchase:exec".into() })));
    }
    if check_rate_limit(&state.rate_limiter, &key.key_id).is_err() {
        return Err((StatusCode::TOO_MANY_REQUESTS, Json(ErrorResponse { error: "money endpoint rate limit exceeded".into() })));
    }
    // 1. Fetch mandate
    let mandate = db::mandate_repo::find_by_id(&state.db, body.mandate_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "mandate not found".into() })))?;

    // 2. Policy evaluation (auth signature + expiry + budget + scope)
    let product = state
        .catalog
        .get(&mandate.merchant_id, &body.product_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "product not found".into() })))?;

    let order_amount = product.price * body.quantity;

    // Build and sign a per-purchase authorization (replay-protected by its nonce)
    let mut auth = PurchaseAuth::new(
        mandate.mandate_id,
        order_amount,
        &mandate.currency,
        &body.product_id,
        &product.category,
    );
    state.signer.sign_auth(&mut auth)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    let evaluator = PolicyEvaluator::new(&state.signer);
    let decision = evaluator.evaluate(&mandate, &auth);

    // 3. Audit the decision
    let (decision_str, reason_str) = match &decision {
        policy_engine::Decision::Allow { .. } => ("allow", None),
        policy_engine::Decision::Block { reason, detail } => ("block", Some(format!("{:?}: {}", reason, detail))),
        policy_engine::Decision::Escalate { reason, rule } => ("escalate", Some(format!("{}: {}", reason, rule))),
    };

    db::audit_repo::append(&state.db, &db::audit_repo::AuditParams {
        event_type: "purchase_attempt",
        mandate_id: Some(mandate.mandate_id),
        entity_id: &body.product_id,
        decision: decision_str,
        reason: reason_str.as_deref(),
        detail: Some(serde_json::json!({ "amount": order_amount })),
        actor: &mandate.buyer_agent_id,
    }).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    // 4. Handle non-allow decisions
    match &decision {
        policy_engine::Decision::Allow { .. } => {}
        _ => {
            return Err((StatusCode::FORBIDDEN, Json(ErrorResponse {
                error: reason_str.unwrap_or_default(),
            })));
        }
    }

    // 5. Atomically consume the auth nonce and debit the budget (replay + double-spend prevention)
    db::mandate_repo::consume_and_increment_spent(&state.db, mandate.mandate_id, &auth.nonce, order_amount).await
        .map_err(|e| (StatusCode::CONFLICT, Json(ErrorResponse { error: e.to_string() })))?;

    // 6. Audit: budget debited (the actual money movement commitment)
    db::audit_repo::append(&state.db, &db::audit_repo::AuditParams {
        event_type: "budget_debited",
        mandate_id: Some(mandate.mandate_id),
        entity_id: &mandate.mandate_id.to_string(),
        decision: "allow",
        reason: None,
        detail: Some(serde_json::json!({
            "amount_debited": order_amount,
            "spent_total": mandate.spent_amount + order_amount,
            "max_amount": mandate.max_amount,
        })),
        actor: &mandate.buyer_agent_id,
    }).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    // 7. Record intent — what the agent intended (the price it saw when it
    //    selected the product). Reconciliation compares this against the
    //    price actually charged to detect drift.
    let intent_id = Uuid::new_v4();
    let expected_price = body.expected_price * body.quantity;
    db::intent_repo::insert(
        &state.db,
        intent_id,
        mandate.mandate_id,
        &body.product_id,
        &auth.category,
        expected_price,
        &mandate.currency,
        body.reasoning.as_deref(),
        body.selection_method.as_deref(),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    // 8. Create order via Razorpay at the live (possibly drifted) price
    let receipt = mandate.mandate_id.to_string();
    let order = state.razorpay.create_order(
        order_amount, &mandate.currency, Some(&receipt),
    ).await.map_err(|e| (StatusCode::BAD_GATEWAY, Json(ErrorResponse { error: e.to_string() })))?;

    // Link the intent to the order it produced (order_id is only known now).
    db::intent_repo::link_to_order(&state.db, intent_id, &order.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    db::order_repo::insert(&state.db, &order.id, mandate.mandate_id,
        order_amount, &mandate.currency, &order.status, Some(&receipt)).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    // 9. Audit: order created
    db::audit_repo::append(&state.db, &db::audit_repo::AuditParams {
        event_type: "order_created",
        mandate_id: Some(mandate.mandate_id),
        entity_id: &order.id,
        decision: "allow",
        reason: None,
        detail: Some(serde_json::json!({ "amount": order_amount })),
        actor: &mandate.buyer_agent_id,
    }).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    // 10. Reconcile: compare the intent (expected price) against the actual
    //     outcome (the order we just charged). Detects catalog drift and
    //     triggers recovery if the prices diverge.
    let outcome = reconciliation::Outcome {
        order_id: order.id.clone(),
        payment_id: String::new(), // no captured payment in this flow
        product_id: body.product_id.clone(),
        actual_price: order_amount,
        currency: mandate.currency.clone(),
        completed_at: Utc::now(),
    };
    let intent = reconciliation::Intent {
        intent_id,
        mandate_id: mandate.mandate_id,
        product_id: body.product_id.clone(),
        category: auth.category.clone(),
        expected_price,
        currency: mandate.currency.clone(),
        created_at: Utc::now(),
    };

    state.reconcile.reconcile_purchase(&intent, &outcome, &mandate.buyer_agent_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok((
        StatusCode::CREATED,
        Json(PurchaseResponse {
            order_id: order.id,
            payment_id: None,
            status: order.status,
            amount: order_amount,
        }),
    ))
}

async fn revoke_mandate(
    State(state): State<AppState>,
    Json(body): Json<RevokeMandateBody>,
) -> Result<Json<RevokeMandateResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 1. Fetch mandate
    let mandate = db::mandate_repo::find_by_id(&state.db, body.mandate_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "mandate not found".into() })))?;

    // 2. Update status to revoked
    db::mandate_repo::update_status(&state.db, mandate.mandate_id, mandate_engine::MandateStatus::Revoked).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    // 3. Audit: mandate revoked
    db::audit_repo::append(&state.db, &db::audit_repo::AuditParams {
        event_type: "mandate_revoked",
        mandate_id: Some(mandate.mandate_id),
        entity_id: &mandate.mandate_id.to_string(),
        decision: "allow",
        reason: body.reason.as_deref(),
        detail: Some(serde_json::json!({
            "previous_status": "active",
            "new_status": "revoked",
        })),
        actor: &mandate.user_id,
    }).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok(Json(RevokeMandateResponse {
        mandate_id: mandate.mandate_id,
        status: "revoked".into(),
    }))
}

async fn get_audit_trail(
    State(state): State<AppState>,
    Path(mandate_id): Path<Uuid>,
) -> Result<Json<AuditTrailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let entries = db::audit_repo::find_by_mandate(&state.db, mandate_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok(Json(AuditTrailResponse {
        mandate_id,
        entries: entries.into_iter().map(|e| AuditEntryResponse {
            event_type: e.event_type,
            entity_id: e.entity_id,
            decision: e.decision,
            reason: e.reason,
            detail: e.detail,
            actor: e.actor,
            created_at: e.created_at,
        }).collect(),
    }))
}

#[derive(Deserialize)]
struct ListAuditQuery {
    mandate_id: Option<String>,
    event_type: Option<String>,
    decision: Option<String>,
    limit: Option<i64>,
}

/// Operator feed: recent audit events, optionally filtered. Powers the live
/// audit stream in the dashboard.
async fn list_audit(
    auth::AuthUser(_): auth::AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListAuditQuery>,
) -> Result<Json<Vec<AuditEntryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(200).clamp(1, 1000);
    let mut rows = db::audit_repo::list_recent(&state.db, limit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    if let Some(m) = &params.mandate_id {
        rows.retain(|r| r.mandate_id.map(|u| u.to_string()).as_deref() == Some(m.as_str()));
    }
    if let Some(et) = &params.event_type {
        rows.retain(|r| &r.event_type == et);
    }
    if let Some(d) = &params.decision {
        rows.retain(|r| &r.decision == d);
    }

    let items = rows
        .into_iter()
        .map(|e| AuditEntryResponse {
            event_type: e.event_type,
            entity_id: e.entity_id,
            decision: e.decision,
            reason: e.reason,
            detail: e.detail,
            actor: e.actor,
            created_at: e.created_at,
        })
        .collect();
    Ok(Json(items))
}

#[derive(Serialize)]
struct MismatchResponse {
    mismatch_id: Uuid,
    mandate_id: Uuid,
    intent_id: Option<Uuid>,
    kind: serde_json::Value,
    status: String,
    refund_id: Option<String>,
    detected_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
struct OrderView {
    order_id: String,
    mandate_id: Uuid,
    amount: i64,
    currency: String,
    status: String,
    created_at: chrono::DateTime<Utc>,
    fulfillment: String,
    product_id: Option<String>,
    category: Option<String>,
    reasoning: Option<String>,
    selection_method: Option<String>,
}

/// Merchant/admin view of orders with fulfillment status + the buyer intent
/// (product, reasoning) that produced each.
async fn list_orders(
    auth::AuthUser(_claims): auth::AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<OrderView>>, (StatusCode, Json<ErrorResponse>)> {
    let rows = db::order_repo::list_recent(&state.db, 100)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    let items = rows
        .into_iter()
        .map(|r| OrderView {
            fulfillment: if r.fulfilled_at.is_some() { "fulfilled".to_string() } else { "unfulfilled".to_string() },
            order_id: r.order_id,
            mandate_id: r.mandate_id,
            amount: r.amount,
            currency: r.currency,
            status: r.status,
            created_at: r.created_at,
            product_id: r.product_id,
            category: r.category,
            reasoning: r.reasoning,
            selection_method: r.selection_method,
        })
        .collect();

    Ok(Json(items))
}

/// Operator view of intent-vs-outcome mismatches (reconciliation).
async fn list_mismatches(
    auth::AuthUser(_): auth::AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<MismatchResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let rows = db::mismatch_repo::list_recent(&state.db, 100)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    let items = rows
        .into_iter()
        .map(|r| MismatchResponse {
            mismatch_id: r.mismatch_id,
            mandate_id: r.mandate_id,
            intent_id: r.intent_id,
            kind: r.kind,
            status: r.status,
            refund_id: r.refund_id,
            detected_at: r.detected_at,
        })
        .collect();
    Ok(Json(items))
}

/// Single integer count helper for admin metrics.
async fn count(
    pool: &PgPool,
    q: &str,
) -> Result<i64, (StatusCode, Json<ErrorResponse>)> {
    sqlx::query_scalar::<_, i64>(q)
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))
}

#[derive(Serialize)]
struct AdminMetrics {
    total_mandates: i64,
    active_mandates: i64,
    total_orders: i64,
    captured_volume_paise: i64,
    blocked_decisions: i64,
    mismatches: i64,
    unresolved_mismatches: i64,
    unfulfilled_orders: i64,
    merchants: i64,
    agents: i64,
    admins: i64,
    active_api_keys: i64,
}

/// Platform-wide metrics for the admin console. Admin role only.
async fn admin_metrics(
    auth::AuthUser(claims): auth::AuthUser,
    State(state): State<AppState>,
) -> Result<Json<AdminMetrics>, (StatusCode, Json<ErrorResponse>)> {
    if claims.role != "admin" {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse { error: "admin role required".into() })));
    }
    let total_mandates = count(&state.db, "SELECT COUNT(*) FROM mandates").await?;
    let active_mandates = count(&state.db, "SELECT COUNT(*) FROM mandates WHERE status = 'active'").await?;
    let total_orders = count(&state.db, "SELECT COUNT(*) FROM orders").await?;
    let captured_volume_paise = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(amount)::bigint, 0) FROM payments WHERE status = 'captured'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
    let blocked_decisions = count(&state.db, "SELECT COUNT(*) FROM audit_log WHERE decision = 'blocked'").await?;
    let mismatches = count(&state.db, "SELECT COUNT(*) FROM mismatches").await?;
    let unresolved_mismatches = count(
        &state.db,
        "SELECT COUNT(*) FROM mismatches WHERE status <> 'refund_completed'",
    )
    .await?;
    let unfulfilled_orders = count(
        &state.db,
        "SELECT COUNT(*) FROM orders o LEFT JOIN fulfillments f ON o.order_id = f.order_id WHERE f.order_id IS NULL",
    )
    .await?;
    let merchants = count(&state.db, "SELECT COUNT(*) FROM accounts WHERE role = 'merchant'").await?;
    let agents = count(&state.db, "SELECT COUNT(*) FROM accounts WHERE role = 'agent'").await?;
    let admins = count(&state.db, "SELECT COUNT(*) FROM accounts WHERE role = 'admin'").await?;
    let active_api_keys = count(&state.db, "SELECT COUNT(*) FROM api_keys WHERE revoked = false").await?;

    Ok(Json(AdminMetrics {
        total_mandates,
        active_mandates,
        total_orders,
        captured_volume_paise,
        blocked_decisions,
        mismatches,
        unresolved_mismatches,
        unfulfilled_orders,
        merchants,
        agents,
        admins,
        active_api_keys,
    }))
}

// ─── Reconciliation: fulfillment ingestion ──────────────────────────────

#[derive(Deserialize)]
struct FulfillmentIngest {
    order_id: String,
    /// Opaque proof of delivery: tracking number, delivery webhook, etc.
    proof: String,
}

/// Merchant posts proof that an order was fulfilled. Marks the order delivered
/// so the verify-then-pay sweep will not recover it. Idempotent.
async fn ingest_fulfillment(
    auth::AuthUser(claims): auth::AuthUser,
    State(state): State<AppState>,
    Json(body): Json<FulfillmentIngest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if claims.role != "admin" && claims.role != "merchant" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "merchant or admin role required".into(),
            }),
        ));
    }
    state
        .reconcile
        .ingest_fulfillment(&body.order_id, &body.proof)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;
    Ok(Json(serde_json::json!({ "status": "fulfilled" })))
}

// ─── Auth handlers ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SignupBody {
    role: String,        // "merchant" | "agent" | "admin"
    email: String,
    name: String,
    password: String,
    /// For merchant: the merchant_id they operate (defaults to a generated id).
    /// For agent: the agent_id. Ignored for admin.
    tenant_id: Option<String>,
}

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    token: String,
    role: String,
    account_id: String,
    tenant_id: Option<String>,
}

async fn signup(
    State(state): State<AppState>,
    Json(body): Json<SignupBody>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let role = body.role.as_str();
    if !matches!(role, "merchant" | "agent" | "admin") {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "invalid role".into() })));
    }
    // Admins have no tenant; merchants/agents must declare one.
    let tenant_id = match role {
        "admin" => None,
        _ => Some(body.tenant_id.unwrap_or_else(|| format!("{}-{}", role, uuid::Uuid::new_v4()))),
    };

    let account = db::auth_repo::AuthRepo::new(state.db.clone())
        .create_account(role, &body.email, &body.name, &body.password, tenant_id.as_deref())
        .await
        .map_err(|e| (StatusCode::CONFLICT, Json(ErrorResponse { error: format!("signup failed: {e}") })))?;

    let claims = auth::Claims {
        sub: account.account_id.to_string(),
        role: account.role,
        tenant_id: account.tenant_id,
        exp: (chrono::Utc::now() + chrono::Duration::hours(12)).timestamp(),
    };
    let token = auth::issue_token(&state.jwt_secret, &claims)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok(Json(AuthResponse {
        token,
        role: claims.role,
        account_id: claims.sub,
        tenant_id: claims.tenant_id,
    }))
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = db::auth_repo::AuthRepo::new(state.db.clone());
    let account = repo
        .verify_password(&body.email, &body.password)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "invalid credentials".into() })))?;

    let claims = auth::Claims {
        sub: account.account_id.to_string(),
        role: account.role,
        tenant_id: account.tenant_id,
        exp: (chrono::Utc::now() + chrono::Duration::hours(12)).timestamp(),
    };
    let token = auth::issue_token(&state.jwt_secret, &claims)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok(Json(AuthResponse {
        token,
        role: claims.role,
        account_id: claims.sub,
        tenant_id: claims.tenant_id,
    }))
}

async fn whoami(auth::AuthUser(claims): auth::AuthUser) -> Json<auth::Claims> {
    Json(claims)
}

/// True if the API key's `scopes` JSON array contains `scope`.
fn has_scope(key: &db::api_key_repo::ApiKeyRow, scope: &str) -> bool {
    key.scopes
        .as_array()
        .map(|a| a.iter().any(|v| v.as_str() == Some(scope)))
        .unwrap_or(false)
}

// ─── Agent API-key handlers ───────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateKeyBody {
    label: String,
    scopes: Vec<String>,
}

#[derive(Serialize)]
struct CreateKeyResponse {
    key: String,           // raw key, shown once
    key_id: String,
    label: String,
    scopes: Vec<String>,
}

#[derive(Serialize)]
struct KeyInfo {
    key_id: String,
    label: String,
    scopes: Vec<String>,
    tenant_id: Option<String>,
    revoked: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Issues a scoped API key for the authenticated account.
async fn create_agent_key(
    auth::AuthUser(claims): auth::AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateKeyBody>,
) -> Result<Json<CreateKeyResponse>, (StatusCode, Json<ErrorResponse>)> {
    let account_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() })))?;

    let (raw, row) = db::api_key_repo::ApiKeyRepo::new(state.db.clone())
        .create_key(account_id, &body.label, body.scopes.clone(), claims.tenant_id.as_deref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok(Json(CreateKeyResponse {
        key: raw,
        key_id: row.key_id.to_string(),
        label: row.label,
        scopes: body.scopes,
    }))
}

/// Lists the authenticated account's API keys (never returns raw secrets).
async fn list_agent_keys(
    auth::AuthUser(claims): auth::AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<KeyInfo>>, (StatusCode, Json<ErrorResponse>)> {
    let account_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() })))?;

    let rows = db::api_key_repo::ApiKeyRepo::new(state.db.clone())
        .list_for_account(&account_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    let infos = rows
        .into_iter()
        .map(|r| KeyInfo {
            key_id: r.key_id.to_string(),
            label: r.label,
            scopes: r.scopes.as_array().map(|a| {
                a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
            }).unwrap_or_default(),
            tenant_id: r.tenant_id,
            revoked: r.revoked,
            created_at: r.created_at,
            last_used_at: r.last_used_at,
        })
        .collect();
    Ok(Json(infos))
}

/// Revokes one of the authenticated account's API keys.
async fn revoke_agent_key(
    auth::AuthUser(claims): auth::AuthUser,
    State(state): State<AppState>,
    Path(key_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let account_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() })))?;

    let ok = db::api_key_repo::ApiKeyRepo::new(state.db.clone())
        .revoke(key_id, account_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    if ok {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, Json(ErrorResponse { error: "key not found".into() })))
    }
}

// ─── Sample Data (for demo) ──────────────────────────────────────────────

fn sample_manifest() -> MerchantManifest {
    MerchantManifest {
        merchant_id: "merchant-001".into(),
        merchant_name: "TechStore Demo".into(),
        capability_version: "1.0".into(),
        supported_scopes: vec!["electronics".into(), "accessories".into()],
        catalog_endpoint: "http://localhost:8000/catalog".into(),
        mandate_endpoint: "http://localhost:8000/mandate".into(),
    }
}

/// Maps a persisted catalog row to the ACP-shaped `Product` served by the API.
fn row_to_product(row: db::catalog_repo::CatalogProductRow) -> Product {
    Product {
        product_id: row.product_id,
        offer_id: row.offer_id,
        title: row.title,
        description: row.description,
        category: row.category,
        price: row.price,
        currency: row.currency,
        availability: match row.availability.as_str() {
            "out_of_stock" => Availability::OutOfStock,
            "preorder" => Availability::Preorder,
            _ => Availability::InStock,
        },
        inventory_count: row.inventory_count,
        seller_name: row.seller_name,
        seller_id: row.merchant_id,
        updated_at: row.updated_at,
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/mandate_gateway".into());

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to database");

    let key_id = std::env::var("RAZORPAY_KEY_ID").unwrap_or_default();
    let key_secret = std::env::var("RAZORPAY_KEY_SECRET").unwrap_or_default();
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        tracing::warn!("JWT_SECRET not set — generating an ephemeral signing key (tokens invalid after restart)");
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    })
    .into_bytes();

    // ---- Signing-key persistence at rest ---------------------------------
    // The Ed25519 signing key must survive restarts, otherwise every
    // previously issued mandate becomes unverifiable. We generate once,
    // encrypt it at rest via the keychain (AES-GCM under a master secret
    // from env), and reload the same key on every subsequent boot. The
    // master secret never touches the database or git.
    let master_secret = std::env::var("MANDATE_MASTER_KEY")?;
    let keychain = db::keychain_repo::KeychainRepo::new(db.clone());

    let (_candidate, fresh_key) = MandateSigner::generate();
    let key_id_hex = hex::encode(fresh_key.verifying_key().to_bytes());
    let persisted_key = keychain
        .load_or_create(master_secret.as_bytes(), fresh_key.to_bytes(), &key_id_hex)
        .await
        .map_err(|e| format!("failed to load/store signing keychain: {e}"))?;
    let signer = Arc::new(MandateSigner::from_key_bytes(persisted_key));

    let razorpay = RazorpayClient::new(&key_id, &key_secret);

    let catalog = db::catalog_repo::CatalogRepo::new(db.clone());
    let merchant = db::merchant_repo::MerchantRepo::new(db.clone());
    let merchant_id = std::env::var("MERCHANT_ID").unwrap_or_else(|_| "merchant-001".into());
    let reconcile = ReconcileService::new(db.clone(), RazorpayRefundProvider(razorpay.clone()));
    let reconcile_for_sweep = reconcile.clone();

    let state = AppState {
        db,
        signer,
        razorpay: Arc::new(razorpay),
        catalog,
        merchant,
        merchant_id,
        jwt_secret,
        reconcile,
        catalog_cache: Arc::new(Mutex::new(None)),
        rate_limiter: Arc::new(Mutex::new(HashMap::new())),
    };

    // Background verify-then-pay sweep: recover orders with no fulfillment
    // proof within the SLA. Independent of the request path.
    tokio::spawn(async move {
        let mut iv = tokio::time::interval(std::time::Duration::from_secs(30));
        iv.tick().await; // consume the immediate first tick
        loop {
            iv.tick().await;
            match reconcile_for_sweep.sweep_unfulfilled(240, "system").await {
                Ok(n) if n > 0 => tracing::info!(recovered = n, "reconciliation: fulfillment-timeout sweep"),
                Ok(_) => {}
                Err(e) => tracing::error!(error = %e, "reconciliation: sweep failed"),
            }
        }
    });

    let app = Router::new()
        .route("/manifest", get(get_manifest))
        .route("/catalog", get(get_catalog_cached))
        .route("/catalog/{id}/price", post(update_catalog_price))
        .route("/mandates", get(list_mandates))
        .route("/mandate", post(issue_mandate))
        .route("/mandate/{id}", get(get_mandate))
        .route("/purchase", post(execute_purchase))
        .route("/mandate/revoke", post(revoke_mandate))
        .route("/audit", get(list_audit))
        .route("/audit/{mandate_id}", get(get_audit_trail))
        .route("/orders", get(list_orders))
        .route("/reconciliation/mismatches", get(list_mismatches))
        .route("/reconciliation/ingest-fulfillment", post(ingest_fulfillment))
        .route("/admin/metrics", get(admin_metrics))
        .route("/admin/simulate-drift", post(simulate_drift))
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/me", get(whoami))
        .route("/agents/keys", post(create_agent_key))
        .route("/agents/keys", get(list_agent_keys))
        .route("/agents/keys/{id}", delete(revoke_agent_key))
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
