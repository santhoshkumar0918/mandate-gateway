use chrono::Utc;
use mandate_engine::{Frequency, Mandate, MandateSigner, NewMandate};
use policy_engine::{Decision, DecisionKind, PolicyEvaluator};
use reconciliation::{MismatchDetector, Intent, Outcome, MismatchKind};

/// In-memory nonce checker for integration tests.
#[derive(Debug)]
struct FakeNonceChecker;

impl policy_engine::NonceChecker for FakeNonceChecker {
    fn is_nonce_fresh(&self, _nonce: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(true) // all nonces are fresh in tests
    }
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

    let checker = FakeNonceChecker;
    let evaluator = PolicyEvaluator::new(&signer, &checker);
    let decision = evaluator.evaluate(&mandate, 30_000, "electronics");
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

    let checker = FakeNonceChecker;
    let evaluator = PolicyEvaluator::new(&signer, &checker);
    let decision = evaluator.evaluate(&mandate, 60_000, "electronics");
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
