# Technical Build Plan — Merchant Agent Gateway

> This document is the single source of truth for *how* we build.
> Every file, every method, every connection is explained.
> If you forget what something does, come back here.

---

## Current State (what we have)

```
gateway/
├── Cargo.toml                     ← workspace root (5 members)
├── crates/
│   ├── mandate-engine/            ← ✅ DONE: Ed25519 signing, Mandate struct, 7 tests
│   ├── policy-engine/             ← ✅ DONE: allow/block/escalate evaluator, 11 tests
│   └── razorpay-client/           ← ✅ DONE: Orders/Payments/Refunds HTTP client, 3 tests
├── services/
│   ├── catalog-service/           ← ⚠️ SCAFFOLD: serves GET /manifest, GET /catalog only
│   └── reconciliation/            ← ✅ DONE: mismatch detection, 5 tests
├── Dockerfile                     ← ⚠️ only builds catalog-service
audit/
└── migrations/
    └── 001_initial.sql            ← ✅ DONE: 7 tables, 12 indexes
dashboard/                         ← ⚠️ SCAFFOLD: default create-next-app page
buyer-agent/                       ← ⚠️ SCAFFOLD: hits non-existent POST endpoints
tests/integration/                 ← ⚠️ 3 tests, separate workspace
```

**The individual building blocks are solid. The pieces are not connected.**

---

## What We Need to Build (the plan)

### Build Order (dependency-driven)

```
Phase 1: DB Layer (no deps on anything else)
    ↓
Phase 2: Gateway Server (depends on Phase 1 + existing crates)
    ↓
Phase 3: Consent UI (depends on Phase 2 API)
    ↓
Phase 4: Buyer Agent Integration (depends on Phase 2 API)
    ↓
Phase 5: Reconciliation Flow (depends on Phase 2 + razorpay-client)
    ↓
Phase 6: Catalog Drift Simulation (depends on Phase 5)
    ↓
Phase 7: E2E Tests + Polish (depends on everything)
```

---

## Phase 1: Database Layer

### Why this comes first

Every non-negotiable rule in AGENTS.md requires audit log writes. The mandate table needs persistence. The reconciliation flow needs row-level locking. None of this works without a DB layer. This is the foundation everything else plugs into.

### What we're building

A new `db` crate at `gateway/crates/db/` that owns all Postgres interaction. Other crates never touch SQL directly — they call repo methods.

### Files

#### `gateway/crates/db/Cargo.toml`
```
Dependencies:
- sqlx (with postgres feature, runtime-tokio, tls-rustls)
- uuid
- chrono
- serde, serde_json
- thiserror
- tokio (for runtime)

Dev-dependencies:
- sqlx (with runtime-tokio for migrations in tests)
```

**Why sqlx:** Compile-time checked queries. If the schema changes and the query doesn't, it won't compile. This catches bugs at build time, not runtime. The proposal explicitly calls for this.

**Why a separate crate:** Isolation. The gateway server, policy-engine, and reconciliation crate don't need to know about Postgres. They depend on the `db` crate's repo traits, not raw SQL. This is the "ports & adapters" pattern — the DB is an adapter, not a core dependency.

#### `gateway/crates/db/src/lib.rs`
Module root. Exports:
- `PgPool` (re-exported from sqlx)
- `MandateRepo`
- `AuditRepo`
- `OrderRepo`
- `PaymentRepo`
- `RefundRepo`
- `IntentRepo`
- `MismatchRepo`
- `DbError` (all DB errors in one enum)

#### `gateway/crates/db/src/error.rs`
```rust
pub enum DbError {
    Connection(String),      // Pool creation failed
    Query(String),           // Any sqlx query error
    NotFound,                // Row not found (optional, for find-by-id)
    Serialization(String),   // JSONB serialization failed
}
```
**Why one error enum:** Every repo returns the same error type. The gateway server handles one error type, not seven. Simpler error handling in handlers.

#### `gateway/crates/db/src/mandate_repo.rs`
Handles all mandate CRUD.

**Methods:**
```rust
impl MandateRepo {
    pub async fn insert(&self, pool: &PgPool, mandate: &Mandate) -> Result<(), DbError>
    pub async fn find_by_id(&self, pool: &PgPool, id: Uuid) -> Result<Option<Mandate>, DbError>
    pub async fn find_by_nonce(&self, pool: &PgPool, nonce: &str) -> Result<Option<Mandate>, DbError>
    pub async fn increment_spent(&self, pool: &PgPool, id: Uuid, amount: i64) -> Result<(), DbError>
    pub async fn update_status(&self, pool: &PgPool, id: Uuid, status: MandateStatus) -> Result<(), DbError>
    pub async fn list_by_merchant(&self, pool: &PgPool, merchant_id: &str) -> Result<Vec<Mandate>, DbError>
}
```

**Why `increment_spent` is important:** This is the atomic counter. When a payment executes, we atomically increment `spent_amount` on the mandate row. This prevents double-spend — two concurrent purchases can't both see `spent_amount=0` and both succeed. Postgres handles this atomically.

**Why `find_by_nonce`:** For replay protection. Before accepting a mandate, check if its nonce has been used. The DB has a UNIQUE constraint on nonce, so this is both a application-level check and a DB-level safety net.

**SQL pattern (insert example):**
```sql
INSERT INTO mandates (mandate_id, issued_at, expires_at, user_id, merchant_id,
    buyer_agent_id, max_amount, currency, scope, frequency, spent_amount,
    status, nonce, signature)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
```
All parameters are positional ($1-$14) — sqlx validates these against the schema at compile time.

#### `gateway/crates/db/src/audit_repo.rs`
Append-only audit log writer.

**Methods:**
```rust
impl AuditRepo {
    pub async fn append(
        &self,
        pool: &PgPool,
        event_type: &str,
        mandate_id: Option<Uuid>,
        entity_id: &str,
        decision: &str,
        reason: Option<&str>,
        detail: Option<serde_json::Value>,
        actor: &str,
    ) -> Result<(), DbError>
}
```

**Why `append` not `insert`:** Naming signals intent. This method is append-only — there's no `update` or `delete`. The audit log is immutable. The method name makes this contract visible in code.

**Why `decision` is a string, not an enum:** The DB stores 'allow', 'block', 'escalate' as CHECK-constrained text. The Rust enum is `DecisionKind`. Converting at the boundary keeps the DB schema simple and the Rust types rich.

**SQL:**
```sql
INSERT INTO audit_log (event_type, mandate_id, entity_id, decision, reason, detail, actor)
VALUES ($1, $2, $3, $4, $5, $6, $7)
```

#### `gateway/crates/db/src/order_repo.rs`
```rust
impl OrderRepo {
    pub async fn insert(&self, pool: &PgPool, order: &Order, mandate_id: Uuid) -> Result<(), DbError>
    pub async fn find_by_id(&self, pool: &PgPool, order_id: &str) -> Result<Option<Order>, DbError>
}
```

#### `gateway/crates/db/src/payment_repo.rs`
```rust
impl PaymentRepo {
    pub async fn insert(&self, pool: &PgPool, payment: &Payment, mandate_id: Uuid) -> Result<(), DbError>
    pub async fn find_by_id(&self, pool: &PgPool, payment_id: &str) -> Result<Option<Payment>, DbError>
}
```

#### `gateway/crates/db/src/intent_repo.rs`
```rust
impl IntentRepo {
    pub async fn insert(&self, pool: &PgPool, intent: &Intent) -> Result<(), DbError>
    pub async fn find_by_order_id(&self, pool: &PgPool, order_id: &str) -> Result<Option<Intent>, DbError>
}
```

#### `gateway/crates/db/src/mismatch_repo.rs`
```rust
impl MismatchRepo {
    pub async fn insert(&self, pool: &PgPool, mismatch: &Mismatch) -> Result<(), DbError>
    pub async fn update_status(
        &self, pool: &PgPool, id: Uuid,
        status: MismatchStatus, refund_id: Option<&str>
    ) -> Result<(), DbError>
    pub async fn find_by_mandate(&self, pool: &PgPool, mandate_id: Uuid) -> Result<Vec<Mismatch>, DbError>
}
```

**Why `update_status` takes `refund_id`:** When a refund is initiated, we update the mismatch status AND store the refund ID in one call. This is the reconciliation lifecycle: detected → refund_initiated (with refund_id) → refund_completed.

#### `gateway/crates/db/src/refund_repo.rs`
```rust
impl RefundRepo {
    pub async fn insert(&self, pool: &PgPool, refund: &Refund, mandate_id: Uuid, idempotency_key: &str) -> Result<(), DbError>
    pub async fn find_by_idempotency_key(&self, pool: &PgPool, key: &str) -> Result<Option<Refund>, DbError>
}
```

**Why idempotency_key:** The refunds table has a UNIQUE constraint on idempotency_key. If a refund is triggered twice (retry, duplicate webhook), the second insert fails with a unique violation, preventing double refunds. The `find_by_idempotency_key` method lets us check first.

### Tests

Add `sqlx` with `offline` feature for compile-time query checking. For unit tests, use a test Postgres instance (via `sqlx::test` macro or Docker).

**Minimum tests for Phase 1:**
1. Insert and retrieve a mandate
2. Increment spent_amount atomically
3. Append audit log entry and verify it exists
4. Insert order, payment, refund with FK relationships
5. Nonce uniqueness constraint violation

### Cargo.toml update

Add `db` to workspace members:
```toml
members = [
    "crates/mandate-engine",
    "crates/policy-engine",
    "crates/razorpay-client",
    "crates/db",                    # ← NEW
    "services/catalog-service",
    "services/reconciliation",
]
```

### Commit
```
db: add sqlx Postgres layer with mandate, audit, order, payment, refund, intent, mismatch repos
```

---

## Phase 2: Gateway Server (Unified HTTP API)

### Why this comes second

The buyer-agent needs endpoints to call. The consent UI needs an API to hit. The reconciliation flow needs HTTP triggers. This is the central hub that connects everything.

### What we're building

Expand `catalog-service` into a full `gateway-server`. Same binary, more routes.

### Files to modify

#### `gateway/services/catalog-service/Cargo.toml`
Add dependencies:
```toml
mandate-engine = { path = "../../crates/mandate-engine" }
policy-engine = { path = "../../crates/policy-engine" }
razorpay-client = { path = "../../crates/razorpay-client" }
db = { path = "../../crates/db" }
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono"] }
dotenvy = "0.15"
```

**Why dotenvy:** Loads `.env` file into environment variables. Standard Rust approach. The `.env.example` already defines the vars — dotenvy reads them at startup.

#### `gateway/services/catalog-service/src/main.rs`
Complete rewrite. The current `main.rs` serves hardcoded JSON. The new one:

```rust
// main.rs structure:
// 1. Load .env
// 2. Initialize tracing
// 3. Connect to Postgres (PgPool)
// 4. Generate or load Ed25519 signing key
// 5. Build axum Router with all routes
// 6. Start server

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await.expect("Failed to connect to Postgres");

    let key_id = std::env::var("RAZORPAY_KEY_ID").expect("RAZORPAY_KEY_ID must be set");
    let key_secret = std::env::var("RAZORPAY_KEY_SECRET").expect("RAZORPAY_KEY_SECRET must be set");

    // Generate signing key (in production, load from secure storage)
    let (signer, _signing_key) = MandateSigner::generate();

    let state = AppState {
        pool,
        signer,
        razorpay_orders: OrdersApi::new(&key_id, &key_secret),
        razorpay_payments: PaymentsApi::new(&key_id, &key_secret),
        razorpay_refunds: RefundsApi::new(&key_id, &key_secret),
    };

    let app = Router::new()
        // Existing routes
        .route("/manifest", get(get_manifest))
        .route("/catalog", get(get_catalog))
        // New routes
        .route("/mandate", post(create_mandate))
        .route("/purchase", post(execute_purchase))
        .route("/audit/{mandate_id}", get(get_audit_trail))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

**Why `AppState`:** Axum's `with_state` pattern. Every handler receives a clone of AppState (cheap — Arc internally). This is how handlers access the DB pool, signer, and Razorpay clients without globals.

**Why `generate()` for signing key:** For the demo, we generate a fresh keypair on each startup. In production, you'd load from a secure store (AWS KMS, etc.). The demo doesn't need persistence here — the mandate's signature is what matters, and it's stored in the DB.

#### `gateway/services/catalog-service/src/handlers/mandate.rs`
POST /mandate handler.

**Request body:**
```json
{
  "product_id": "prod-001",
  "amount": 129900,
  "category": "electronics"
}
```

**What it does:**
1. Parse JSON body into `CreateMandateRequest`
2. Create `NewMandate` with user_id (hardcoded "demo-user" for now), merchant_id, buyer_agent_id
3. Call `Mandate::new(params)` → unsigned mandate
4. Call `signer.sign(&mut mandate)` → signed mandate
5. Call `MandateRepo::insert(pool, &mandate)` → persist to DB
6. Call `AuditRepo::append(pool, "mandate_issued", ...)` → audit log
7. Return `Json(mandate)` to caller

**Why audit happens here:** The mandate issuance is a money-adjacent action (it creates the permission to move money). Per AGENTS.md rule #3, every money-moving action writes an audit entry. This is the first audit entry in the flow.

**Error handling:** All errors return structured JSON with appropriate HTTP status codes (400 for bad input, 500 for DB errors). No `.unwrap()` on any error path.

#### `gateway/services/catalog-service/src/handlers/purchase.rs`
POST /purchase handler.

**Request body:**
```json
{
  "mandate_id": "uuid-here",
  "product_id": "prod-001",
  "amount": 129900
}
```

**What it does:**
1. Parse JSON body into `PurchaseRequest`
2. Fetch mandate from DB: `MandateRepo::find_by_id(pool, mandate_id)`
3. Evaluate policy: `PolicyEvaluator::new(&signer).evaluate(&mandate, amount, category)`
4. If Decision::Block → return 403 with block reason, write audit log
5. If Decision::Allow → proceed:
   a. Create Razorpay order: `razorpay_orders.create_order(amount, "INR", Some(&order_id))`
   b. Capture payment: `razorpay_payments.capture_payment(&order_id, amount, "INR", "upi")`
   c. Insert order in DB: `OrderRepo::insert(pool, &order, mandate_id)`
   d. Insert payment in DB: `PaymentRepo::insert(pool, &payment, mandate_id)`
   e. Increment mandate spent: `MandateRepo::increment_spent(pool, mandate_id, amount)`
   f. Write audit log: `AuditRepo::append(pool, "payment_executed", ...)`
   g. Return `Json(purchase_response)`

**Why this order matters:**
- Policy evaluation FIRST — never create an order if the mandate says no
- Order before payment — Razorpay requires an order before a payment
- Audit log AFTER DB writes but BEFORE returning — the audit entry proves the action completed
- `increment_spent` is atomic — prevents double-spend on concurrent requests

**Idempotency:** The order receipt is derived from `mandate_id + product_id + timestamp`. If the same request comes twice, the Razorpay order creation will create a new order (Razorpay doesn't enforce idempotency on orders by default). For the demo, this is acceptable. For production, you'd add an idempotency key.

#### `gateway/services/catalog-service/src/handlers/audit.rs`
GET /audit/:mandate_id handler.

**What it does:**
1. Parse `mandate_id` from path
2. Query audit_log for all entries with this mandate_id, ordered by created_at
3. Return `Json(Vec<AuditEntry>)`

**Why this exists:** The dashboard needs to display the audit trail. This is the read side of the audit log. The write side is in every other handler.

#### `gateway/services/catalog-service/src/handlers/mod.rs`
Module root. Exports all handlers.

#### `gateway/services/catalog-service/src/state.rs`
```rust
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub signer: MandateSigner,
    pub razorpay_orders: OrdersApi,
    pub razorpay_payments: PaymentsApi,
    pub razorpay_refunds: RefundsApi,
}
```

**Why `Clone`:** Axum requires state to be `Clone`. All fields are either `Arc`-wrapped (PgPool, reqwest::Client) or cheap to clone (MandateSigner holds a SigningKey which is 32 bytes). No performance concern.

### Tests

**Minimum tests for Phase 2:**
1. POST /manifest returns manifest JSON
2. POST /catalog returns catalog JSON
3. POST /mandate creates and signs a mandate, persists to DB
4. POST /purchase with valid mandate → Allow → order created
5. POST /purchase with over-budget mandate → Block → 403
6. GET /audit/:id returns audit entries

### Commit
```
catalog-service: expand to unified gateway server with mandate, purchase, and audit routes
```

---

## Phase 3: Consent UI (Dashboard)

### Why this comes third

The proposal explicitly says "build a real consent UI, don't stub it." This is the most visually impactful part of the pitch — 5 seconds of showing a human authorizing a bounded scope.

### What we're building

A Next.js page at `dashboard/src/app/consent/page.tsx` that:
1. Shows a demo user profile
2. Has a budget slider (₹100 – ₹1000)
3. Has a category picker (electronics, accessories)
4. Has an "Authorize" button
5. On click: POST /mandate to gateway
6. Shows the signed mandate result

### Files

#### `dashboard/src/app/consent/page.tsx`
Client component (needs interactivity — slider, button, state).

**Key decisions:**
- Server component by default (AGENTS.md rule) — but consent page needs `useState` for the slider and button state, so it's a client component
- No client-side storage of mandate or signing material (AGENTS.md rule) — only displays what the API returns
- Fetches from `http://localhost:8000` (or `GATEWAY_URL` env var in Docker)

**UI structure:**
```
┌─────────────────────────────┐
│  Merchant Agent Gateway      │
│  Consent Authorization       │
├─────────────────────────────┤
│  User: Demo User            │
│  Merchant: TechStore Demo   │
│                              │
│  Budget: [----●----] ₹500   │
│  Category: [Electronics ▼]  │
│                              │
│  [Authorize Mandate]         │
├─────────────────────────────┤
│  Mandate issued:            │
│  ID: uuid-xxx               │
│  Max: ₹500                  │
│  Scope: [electronics]       │
│  Expires: 2026-08-27 16:00  │
│  Signature: 0x3a8f...       │
└─────────────────────────────┘
```

#### `dashboard/src/app/page.tsx`
Update to be a landing page that links to /consent.

#### `dashboard/src/lib/gateway-client.ts`
API client for the gateway.

```typescript
const GATEWAY_URL = process.env.GATEWAY_URL || "http://localhost:8000";

export async function createMandate(params: {
  product_id: string;
  amount: number;
  category: string;
}) {
  const res = await fetch(`${GATEWAY_URL}/mandate`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(params),
  });
  if (!res.ok) throw new Error(`Mandate creation failed: ${res.status}`);
  return res.json();
}

export async function getAuditTrail(mandateId: string) {
  const res = await fetch(`${GATEWAY_URL}/audit/${mandateId}`);
  if (!res.ok) throw new Error(`Audit fetch failed: ${res.status}`);
  return res.json();
}
```

**Why a separate lib file:** Isolation. If the gateway URL changes, one file changes. If we add auth headers later, one file changes. Components import from this, never call fetch directly.

### Commit
```
dashboard: build consent UI with budget slider, category picker, and mandate authorization
```

---

## Phase 4: Buyer Agent Integration

### Why this comes fourth

The buyer-agent script already exists but hits non-existent endpoints. Now that Phase 2 created those endpoints, we wire the buyer-agent to actually work.

### Changes

#### `buyer-agent/main.py`
- Fix HTTP client to handle errors (currently crashes on any failure)
- Add proper JSON parsing
- Add response validation
- Test against running gateway

#### `buyer-agent/Dockerfile`
Already correct — Python slim, no changes needed.

### Commit
```
buyer-agent: fix error handling and validate against live gateway endpoints
```

---

## Phase 5: Reconciliation Flow

### Why this comes fifth

The mismatch detection logic exists but has no refund orchestration. This phase wires: detect mismatch → initiate refund → update status → audit log.

### Files

#### `gateway/services/reconciliation/src/orchestrator.rs` (NEW)
```rust
pub struct ReconciliationOrchestrator {
    pool: PgPool,
    razorpay_refunds: RefundsApi,
}

impl ReconciliationOrchestrator {
    pub async fn process_order(
        &self,
        intent: &Intent,
        outcome: &Outcome,
    ) -> Result<Vec<Mismatch>, ReconciliationError> {
        // 1. Detect mismatches
        let mismatches = MismatchDetector::detect(intent, outcome);

        for m in &mismatches {
            // 2. Persist mismatch
            MismatchRepo::insert(&self.pool, m).await?;

            // 3. If price drift, initiate refund
            if let MismatchKind::PriceDrift { .. } = &m.kind {
                let idempotency_key = m.mismatch_id.to_string();
                let refund = self.razorpay_refunds
                    .create_refund(&outcome.payment_id, Some(m.kind.refund_amount()))
                    .await
                    .map_err(|e| ReconciliationError::RefundFailed(e.to_string()))?;

                // 4. Update mismatch status
                MismatchRepo::update_status(
                    &self.pool, m.mismatch_id,
                    MismatchStatus::RefundInitiated,
                    Some(&refund.id),
                ).await?;

                // 5. Audit log
                AuditRepo::append(
                    &self.pool, "refund_initiated",
                    Some(intent.mandate_id), &refund.id,
                    "allow", None, None, "reconciliation-engine",
                ).await?;
            }
        }

        Ok(mismatches)
    }
}
```

**Why orchestrator is separate from matcher:** Matcher is pure comparison (no DB, no API calls). Orchestrator has side effects (DB writes, Razorpay calls). This separation means matcher stays testable without mocks.

### Commit
```
reconciliation: add orchestrator with refund trigger and audit logging
```

---

## Phase 6: Catalog Drift Simulation

### Why this comes sixth

This is the engineered failure — the demo moment that proves the system works. A background job silently bumps a product price between mandate issuance and execution.

### Files

#### `gateway/services/catalog-service/src/drift.rs` (NEW)
```rust
pub async fn simulate_drift(pool: &PgPool, product_id: &str, new_price: i64) {
    // Wait 5 seconds (simulates time between mandate and purchase)
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Update price in catalog (in-memory for demo, DB for production)
    // The buyer-agent's next purchase will see the drifted price
}
```

**Why 5 seconds:** Long enough for the demo to be visible, short enough to not bore the judges.

### Commit
```
catalog-service: add price drift simulation for engineered failure demo
```

---

## Phase 7: E2E Tests + Polish

### What we're building

Full end-to-end tests that exercise the entire flow:
1. Buyer agent discovers catalog
2. Buyer agent requests mandate
3. Policy engine allows
4. Razorpay order created
5. Payment captured
6. Catalog drift occurs
7. Reconciliation detects mismatch
8. Refund triggered
9. Audit trail shows full chain

### Files

#### `tests/integration/src/e2e.rs` (NEW)
Async tests using tokio + sqlx test Postgres.

### Commits
```
tests: add end-to-end tests covering full purchase flow and catalog drift recovery
```

---

## File-by-File Quick Reference

| File | What it does | Why it exists |
|---|---|---|
| `gateway/Cargo.toml` | Workspace definition | Groups all Rust crates, enables `cargo build --workspace` |
| `crates/mandate-engine/src/mandate.rs` | Mandate struct + signing payload | The trust-critical data structure everything depends on |
| `crates/mandate-engine/src/signing.rs` | Ed25519 sign/verify | Proves mandates weren't tampered with |
| `crates/policy-engine/src/evaluator.rs` | Deterministic allow/block/escalate | The money gate — no payment without passing this |
| `crates/policy-engine/src/rules.rs` | Individual rule checks | Each rule is independently testable |
| `crates/razorpay-client/src/orders.rs` | Razorpay Orders API | Creates orders before payments |
| `crates/razorpay-client/src/payments.rs` | Razorpay Payments API | Captures payments against orders |
| `crates/razorpay-client/src/refunds.rs` | Razorpay Refunds API | Issues refunds for mismatch recovery |
| `crates/db/src/mandate_repo.rs` | Mandate DB operations | Persists mandates, atomic spent counter |
| `crates/db/src/audit_repo.rs` | Audit log writer | Every money action logged here |
| `services/catalog-service/src/main.rs` | Axum server + routes | The central HTTP hub connecting everything |
| `services/catalog-service/src/handlers/mandate.rs` | POST /mandate handler | Signs and persists mandates |
| `services/catalog-service/src/handlers/purchase.rs` | POST /purchase handler | Full purchase flow: policy → Razorpay → audit |
| `services/reconciliation/src/matcher.rs` | Pure mismatch detection | Compares intent vs outcome, no side effects |
| `services/reconciliation/src/orchestrator.rs` | Refund orchestration | Detects mismatch → triggers refund → audits |
| `audit/migrations/001_initial.sql` | Postgres schema | All tables, indexes, constraints |
| `dashboard/src/app/consent/page.tsx` | Consent UI | Human authorizes mandate bounds |
| `buyer-agent/main.py` | Reference buyer agent | Proves the gateway works end-to-end |

---

## Next Step

**Start with Phase 1: DB Layer.** Create `gateway/crates/db/` with sqlx, write the repos, add tests. This is the foundation — nothing else works without it.

Say go when you're ready.
