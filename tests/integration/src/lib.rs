/// Integration tests for the merchant agent gateway.
///
/// This crate exists purely as a test harness, so every item lives in a
/// `#[cfg(test)]` module — there is no production library surface here.
#[cfg(test)]
mod tests {
    use chrono::Utc;
    use mandate_engine::purchase_auth::PurchaseAuth;
    use mandate_engine::{Frequency, Mandate, MandateSigner, NewMandate};
    use policy_engine::{Decision, DecisionKind, PolicyEvaluator};
    use reconciliation::{
        Intent, IssuedRefund, MismatchDetector, MismatchKind, Outcome, ReconcileService,
        RefundProvider,
    };


/// Counts refund calls so tests can assert idempotency — a double refund is
/// a real-money bug this test exists to catch. Each issued refund gets a
/// unique id so parallel tests never collide on a shared PK.
#[derive(Clone)]
struct FakeRefundProvider {
    counter: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl RefundProvider for FakeRefundProvider {
    async fn issue_refund(
        &self,
        _payment_id: &str,
        _amount: Option<i64>,
    ) -> Result<IssuedRefund, reconciliation::ReconciliationError> {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(IssuedRefund {
            id: format!("refund_test_{}", uuid::Uuid::new_v4()),
            status: "processed".into(),
        })
    }
}

fn signed_auth(signer: &MandateSigner, mandate: &Mandate, amount: i64, category: &str) -> PurchaseAuth {
    let mut auth = PurchaseAuth::new(
        mandate.mandate_id,
        amount,
        &mandate.currency,
        "prod-001",
        category,
    );
    signer.sign_auth(&mut auth).unwrap();
    auth
}

/// End-to-end: mandate issuance → policy evaluation → happy path.
#[test]
fn mandate_to_purchase_happy_path() {
    let (signer, _) = MandateSigner::generate();
    let mut mandate = Mandate::new(NewMandate {
        user_id: "user-1".into(),
        merchant_id: "merchant-1".into(),
        buyer_agent_id: "agent-1".into(),
        max_amount: 50_000,
        currency: "INR".into(),
        scope: vec!["electronics".into()],
        frequency: Frequency::OneTime,
        expires_at: Utc::now() + chrono::Duration::hours(1),
    });
    signer.sign(&mut mandate).unwrap();

    let evaluator = PolicyEvaluator::new(&signer);
    let auth = signed_auth(&signer, &mandate, 30_000, "electronics");
    let decision = evaluator.evaluate(&mandate, &auth);
    assert!(matches!(decision, Decision::Allow { .. }));
}

/// End-to-end: mandate → policy blocks over-budget → audit logged.
#[test]
fn mandate_over_budget_blocked() {
    let (signer, _) = MandateSigner::generate();
    let mut mandate = Mandate::new(NewMandate {
        user_id: "user-1".into(),
        merchant_id: "merchant-1".into(),
        buyer_agent_id: "agent-1".into(),
        max_amount: 50_000,
        currency: "INR".into(),
        scope: vec!["electronics".into()],
        frequency: Frequency::OneTime,
        expires_at: Utc::now() + chrono::Duration::hours(1),
    });
    signer.sign(&mut mandate).unwrap();

    let evaluator = PolicyEvaluator::new(&signer);
    let auth = signed_auth(&signer, &mandate, 60_000, "electronics");
    let decision = evaluator.evaluate(&mandate, &auth);
    assert_eq!(decision.kind(), DecisionKind::Block);
}

/// Engineered failure: catalog drift → price mismatch detected → recovery needed.
#[test]
fn catalog_drift_mismatch_detected() {
    let intent = Intent {
        intent_id: uuid::Uuid::new_v4(),
        mandate_id: uuid::Uuid::new_v4(),
        product_id: "prod-001".into(),
        category: "electronics".into(),
        expected_price: 129_900,
        currency: "INR".into(),
        created_at: Utc::now(),
    };

    let outcome = Outcome {
        order_id: "order-001".into(),
        payment_id: "pay-001".into(),
        product_id: "prod-001".into(),
        actual_price: 149_900,
        currency: "INR".into(),
        completed_at: Utc::now(),
    };

    let mismatches = MismatchDetector::detect(&intent, &outcome);
    assert_eq!(mismatches.len(), 1);
    assert!(matches!(
        mismatches[0].kind,
        MismatchKind::PriceDrift { expected: 129_900, actual: 149_900 }
    ));
}

async fn test_pool() -> db::PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://santhoshkumar0918@localhost:5432/mandate_gateway".into());
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("test database must be running and migrated")
}

fn make_signed_mandate() -> (MandateSigner, Mandate) {
    let (signer, _) = MandateSigner::generate();
    let mut mandate = Mandate::new(NewMandate {
        user_id: "user-1".into(),
        merchant_id: "merchant-1".into(),
        buyer_agent_id: "agent-1".into(),
        max_amount: 500_000,
        currency: "INR".into(),
        scope: vec!["electronics".into()],
        frequency: Frequency::OneTime,
        expires_at: Utc::now() + chrono::Duration::hours(1),
    });
    signer.sign(&mut mandate).unwrap();
    (signer, mandate)
}

/// Reconciliation end-to-end: drift detected → persisted → refund issued
/// → audit written. Exercise the real row-lock + idempotency path.
#[tokio::test]
async fn reconciliation_detects_drift_and_issues_refund() {
    let pool = test_pool().await;
    let (_signer, mandate) = make_signed_mandate();
    let order_id = format!("order-drift-{}", uuid::Uuid::new_v4());
    let payment_id = format!("pay-{}", uuid::Uuid::new_v4());
    db::mandate_repo::insert(&pool, &mandate).await.unwrap();
    db::order_repo::insert(&pool, &order_id, mandate.mandate_id, 149_900, "INR", "paid", None)
        .await
        .unwrap();

    let intent = Intent {
        intent_id: uuid::Uuid::new_v4(),
        mandate_id: mandate.mandate_id,
        product_id: "prod-001".into(),
        category: "electronics".into(),
        expected_price: 129_900,
        currency: "INR".into(),
        created_at: Utc::now(),
    };
    db::intent_repo::insert(
        &pool,
        intent.intent_id,
        intent.mandate_id,
        &intent.product_id,
        &intent.category,
        intent.expected_price,
        &intent.currency,
    )
    .await
    .unwrap();

    let outcome = Outcome {
        order_id,
        payment_id,
        product_id: "prod-001".into(),
        actual_price: 149_900,
        currency: "INR".into(),
        completed_at: Utc::now(),
    };

    db::payment_repo::insert(
        &pool,
        &db::payment_repo::InsertPaymentParams {
            payment_id: &outcome.payment_id,
            order_id: &outcome.order_id,
            mandate_id: mandate.mandate_id,
            amount: outcome.actual_price,
            currency: &outcome.currency,
            status: "captured",
            method: None,
            captured: true,
        },
    )
    .await
    .unwrap();

    let refund_counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let svc = ReconcileService::new(
        pool.clone(),
        FakeRefundProvider { counter: refund_counter.clone() },
    );

    let mismatches = svc.reconcile_purchase(&intent, &outcome, "agent-1").await.unwrap();
    assert_eq!(mismatches.len(), 1, "drift should be detected");
    assert!(
        matches!(mismatches[0].kind, MismatchKind::PriceDrift { expected: 129_900, actual: 149_900 }),
        "expected price drift"
    );

    // The drift is persisted as a mismatch row.
    let rows = db::mismatch_repo::find_by_mandate(&pool, mandate.mandate_id).await.unwrap();
    assert_eq!(rows.len(), 1, "exactly one mismatch row");
    assert_eq!(rows[0].status, "refund_completed", "refund lifecycle should complete");

    // One refund issued, audited.
    assert_eq!(
        refund_counter.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "exactly one refund call"
    );

    let audit = db::audit_repo::find_by_mandate(&pool, mandate.mandate_id).await.unwrap();
    let events: Vec<&str> = audit.iter().map(|e| e.event_type.as_str()).collect();
    assert!(events.contains(&"mismatch_detected"), "mismatch_detected audited");
    assert!(events.contains(&"refund_triggered"), "refund_triggered audited");
}

/// Idempotency: re-running reconciliation on the same order does NOT
/// issue a second refund — the row lock + idempotency key short-circuit it.
#[tokio::test]
async fn reconciliation_is_idempotent_no_double_refund() {
    let pool = test_pool().await;
    let (_signer, mandate) = make_signed_mandate();
    let order_id = format!("order-drift-{}", uuid::Uuid::new_v4());
    let payment_id = format!("pay-{}", uuid::Uuid::new_v4());
    db::mandate_repo::insert(&pool, &mandate).await.unwrap();
    db::order_repo::insert(&pool, &order_id, mandate.mandate_id, 249_900, "INR", "paid", None)
        .await
        .unwrap();

    let intent = Intent {
        intent_id: uuid::Uuid::new_v4(),
        mandate_id: mandate.mandate_id,
        product_id: "prod-002".into(),
        category: "accessories".into(),
        expected_price: 200_000,
        currency: "INR".into(),
        created_at: Utc::now(),
    };
    db::intent_repo::insert(
        &pool,
        intent.intent_id,
        intent.mandate_id,
        &intent.product_id,
        &intent.category,
        intent.expected_price,
        &intent.currency,
    )
    .await
    .unwrap();

    let outcome = Outcome {
        order_id,
        payment_id,
        product_id: "prod-002".into(),
        actual_price: 249_900,
        currency: "INR".into(),
        completed_at: Utc::now(),
    };

    db::payment_repo::insert(
        &pool,
        &db::payment_repo::InsertPaymentParams {
            payment_id: &outcome.payment_id,
            order_id: &outcome.order_id,
            mandate_id: mandate.mandate_id,
            amount: outcome.actual_price,
            currency: &outcome.currency,
            status: "captured",
            method: None,
            captured: true,
        },
    )
    .await
    .unwrap();

    let refund_counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let svc = ReconcileService::new(
        pool.clone(),
        FakeRefundProvider { counter: refund_counter.clone() },
    );

    // First reconcile → refund issued.
    svc.reconcile_purchase(&intent, &outcome, "agent-1").await.unwrap();

    // Second reconcile of the SAME order → must NOT double-refund.
    svc.reconcile_purchase(&intent, &outcome, "agent-1").await.unwrap();

    assert_eq!(
        refund_counter.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "idempotency violated: double refund issued"
    );
}
}

/// DB-backed test proving the signing-key persistence invariant: a key
/// written once is reloaded unchanged on a simulated restart, so a mandate
/// signed before the restart still verifies afterwards. Requires a migrated
/// Postgres reachable via `DATABASE_URL`.
#[cfg(test)]
mod db_keychain_tests {
    use mandate_engine::{Mandate, MandateSigner, NewMandate, Frequency};
    use chrono::Utc;
    use db::keychain_repo::KeychainRepo;

    async fn pool() -> db::PgPool {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://santhoshkumar0918@localhost:5432/mandate_gateway".into());
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("test database must be running and migrated")
    }

    #[tokio::test]
    async fn load_or_create_with_restart() {
        let pool = pool().await;
        let repo = KeychainRepo::new(pool.clone());

        // Clear any prior active row so the test is deterministic.
        sqlx::query("DELETE FROM keychain WHERE id='active'")
            .execute(&pool)
            .await
            .unwrap();

        let secret = b"integration-test-master-secret";

        // First boot: persist fresh key A.
        let key_a = repo
            .load_or_create(secret, [0xAA; 32], "key-id-A")
            .await
            .unwrap();
        assert_eq!(key_a, [0xAA; 32]);

        // Sign a mandate with A.
        let signer_a = MandateSigner::from_key_bytes(key_a);
        let mut mandate = Mandate::new(NewMandate {
            user_id: "user-1".into(),
            merchant_id: "merchant-1".into(),
            buyer_agent_id: "agent-1".into(),
            max_amount: 50_000,
            currency: "INR".into(),
            scope: vec!["electronics".into()],
            frequency: Frequency::OneTime,
            expires_at: Utc::now() + chrono::Duration::hours(1),
        });
        signer_a.sign(&mut mandate).unwrap();
        assert!(signer_a.verify(&mandate).unwrap());

        // Simulated restart: try to persist a *different* fresh key B.
        let key_reloaded = repo
            .load_or_create(secret, [0xBB; 32], "key-id-B")
            .await
            .unwrap();
        assert_eq!(
            key_reloaded, key_a,
            "restart must reuse the persisted key, not a freshly generated one"
        );

        // A mandate signed before the restart must still verify.
        let signer_reloaded = MandateSigner::from_key_bytes(key_reloaded);
        assert!(
            signer_reloaded.verify(&mandate).unwrap(),
            "pre-restart mandate must remain verifiable after key reload"
        );

        // Cleanup so subsequent runs stay deterministic.
        sqlx::query("DELETE FROM keychain WHERE id='active'")
            .execute(&pool)
            .await
            .unwrap();
    }
}

/// DB-backed test proving the catalog is the Postgres source of truth, so it
/// survives restarts and is not a per-process in-memory `HashMap`. Requires a
/// migrated Postgres reachable via `DATABASE_URL`.
#[cfg(test)]
mod db_catalog_tests {
    use db::catalog_repo::CatalogRepo;

    async fn pool() -> db::PgPool {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://santhoshkumar0918@localhost:5432/mandate_gateway".into());
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("test database must be running and migrated")
    }

    #[tokio::test]
    async fn catalog_is_sourced_from_postgres() {
        let pool = pool().await;
        let repo = CatalogRepo::new(pool.clone());

        // The seeded merchant-001 catalog must be readable.
        let products = repo.list("merchant-001").await.unwrap();
        assert!(
            !products.is_empty(),
            "seeded catalog must be present in Postgres"
        );

        // A known product resolves and its price is the persisted value.
        let product = repo.get("merchant-001", "prod-001").await.unwrap();
        assert!(product.is_some());
        let product = product.unwrap();
        assert_eq!(product.price, 129_900);

        // A price change is persisted and read back (drift scenario).
        repo.set_price("merchant-001", "prod-001", 199_900)
            .await
            .unwrap();
        let after = repo.get("merchant-001", "prod-001").await.unwrap().unwrap();
        assert_eq!(after.price, 199_900);

        // Restore the seeded value so the demo stays deterministic.
        repo.set_price("merchant-001", "prod-001", 129_900)
            .await
            .unwrap();
    }
}
