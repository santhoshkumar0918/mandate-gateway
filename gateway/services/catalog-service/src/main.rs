use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use db::{PgNonceChecker, PgPool};
use mandate_engine::{Frequency, MandateSigner, NewMandate};
use policy_engine::PolicyEvaluator;
use razorpay_client::RazorpayClient;
use reconciliation::{RazorpayRefundProvider, ReconcileService};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, sync::Arc};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod catalog_store;
mod manifest;
use catalog_store::CatalogStore;
use manifest::{MerchantManifest, Product};

// ─── App State ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    db: PgPool,
    signer: Arc<MandateSigner>,
    nonce_checker: Arc<PgNonceChecker>,
    razorpay: Arc<RazorpayClient>,
    catalog: CatalogStore,
    reconcile: ReconcileService<RazorpayRefundProvider>,
}

// ─── Request / Response Types ─────────────────────────────────────────────

#[derive(Deserialize)]
struct IssueMandateBody {
    user_id: String,
    merchant_id: String,
    buyer_agent_id: String,
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

async fn get_manifest() -> Json<MerchantManifest> {
    Json(sample_manifest())
}

async fn get_catalog(State(state): State<AppState>) -> Json<Vec<Product>> {
    Json(state.catalog.list())
}

/// Simulates catalog price drift for the engineered failure scenario.
///
/// Bumps a product's price so a subsequent purchase charges a different
/// amount than what the buyer agent intended, triggering reconciliation.
async fn simulate_drift(
    State(state): State<AppState>,
    Json(body): Json<SimulateDriftBody>,
) -> Result<Json<SimulateDriftResponse>, (StatusCode, Json<ErrorResponse>)> {
    let old_price = state.catalog.get(&body.product_id).map(|p| p.price);
    let updated = state
        .catalog
        .set_price(&body.product_id, body.new_price)
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
    State(state): State<AppState>,
    Json(body): Json<IssueMandateBody>,
) -> Result<(StatusCode, Json<IssueMandateResponse>), (StatusCode, Json<ErrorResponse>)> {
    let frequency = match body.frequency.as_str() {
        "recurring" => Frequency::Recurring,
        _ => Frequency::OneTime,
    };

    let expires_in_hours = body.expires_in_hours.unwrap_or(1);

    let params = NewMandate {
        user_id: body.user_id,
        merchant_id: body.merchant_id,
        buyer_agent_id: body.buyer_agent_id,
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
    State(state): State<AppState>,
    Json(body): Json<PurchaseRequest>,
) -> Result<(StatusCode, Json<PurchaseResponse>), (StatusCode, Json<ErrorResponse>)> {
    // 1. Fetch mandate
    let mandate = db::mandate_repo::find_by_id(&state.db, body.mandate_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "mandate not found".into() })))?;

    // 2. Policy evaluation (nonce check + signature + expiry + budget + scope)
    let product = state
        .catalog
        .get(&body.product_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "product not found".into() })))?;

    // The expected price is what the product costs in the catalog the moment
    // the purchase executes. If a drift was simulated, this is the drifted
    // price. The Intent (recorded lower) holds what the agent intended.
    let order_amount = product.price * body.quantity;
    let evaluator = PolicyEvaluator::new(&state.signer, &*state.nonce_checker);
    let category = &product.category;
    let decision = evaluator.evaluate(&mandate, order_amount, category);

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

    // 5. Increment spent amount (double-spend prevention)
    db::mandate_repo::increment_spent(&state.db, mandate.mandate_id, order_amount).await
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
    db::intent_repo::insert(&state.db, intent_id, mandate.mandate_id,
        &body.product_id, category, expected_price, &mandate.currency).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    // 8. Create order via Razorpay at the live (possibly drifted) price
    let receipt = mandate.mandate_id.to_string();
    let order = state.razorpay.create_order(
        order_amount, &mandate.currency, Some(&receipt),
    ).await.map_err(|e| (StatusCode::BAD_GATEWAY, Json(ErrorResponse { error: e.to_string() })))?;

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
        category: category.clone(),
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

// ─── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
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

    let (signer, _signing_key) = MandateSigner::generate();
    let razorpay = RazorpayClient::new(&key_id, &key_secret);
    let nonce_checker = PgNonceChecker::new(db.clone());

    let catalog = CatalogStore::new();
    let reconcile = ReconcileService::new(db.clone(), RazorpayRefundProvider(razorpay.clone()));

    let state = AppState {
        db,
        signer: Arc::new(signer),
        nonce_checker: Arc::new(nonce_checker),
        razorpay: Arc::new(razorpay),
        catalog,
        reconcile,
    };

    let app = Router::new()
        .route("/manifest", get(get_manifest))
        .route("/catalog", get(get_catalog))
        .route("/mandate", post(issue_mandate))
        .route("/purchase", post(execute_purchase))
        .route("/mandate/revoke", post(revoke_mandate))
        .route("/audit/{mandate_id}", get(get_audit_trail))
        .route("/admin/simulate-drift", post(simulate_drift))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
