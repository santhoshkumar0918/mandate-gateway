use mandate_engine::{Mandate, MandateSigner};

use crate::decision::{BlockReason, Decision};
use crate::nonce_checker::NonceChecker;
use crate::rules;

pub struct PolicyEvaluator<'a> {
    signer: &'a MandateSigner,
    nonce_checker: &'a dyn NonceChecker,
}

impl<'a> PolicyEvaluator<'a> {
    pub fn new(signer: &'a MandateSigner, nonce_checker: &'a dyn NonceChecker) -> Self {
        Self { signer, nonce_checker }
    }

    pub fn evaluate(&self, mandate: &Mandate, amount: i64, category: &str) -> Decision {
        // 1. Nonce replay check — has this nonce been used before?
        match self.nonce_checker.is_nonce_fresh(&mandate.nonce) {
            Ok(true) => {} // nonce is fresh, continue
            Ok(false) => {
                tracing::warn!(
                    mandate_id = %mandate.mandate_id,
                    nonce = %mandate.nonce,
                    "nonce replay detected — nonce already used"
                );
                return Decision::Block {
                    reason: BlockReason::ReplayDetected,
                    detail: format!("nonce {} already used", mandate.nonce),
                };
            }
            Err(e) => {
                tracing::error!(
                    mandate_id = %mandate.mandate_id,
                    error = %e,
                    "nonce check failed — database error"
                );
                return Decision::Block {
                    reason: BlockReason::ReplayDetected,
                    detail: format!("nonce check failed: {e}"),
                };
            }
        }

        // 2. Signature verification — tampered mandate = hard block
        match self.signer.verify(mandate) {
            Ok(true) => {}
            Ok(false) => {
                tracing::warn!(
                    mandate_id = %mandate.mandate_id,
                    "signature verification failed — possible tampering"
                );
                return Decision::Block {
                    reason: BlockReason::ReplayDetected,
                    detail: "mandate signature verification failed".into(),
                };
            }
            Err(e) => {
                tracing::warn!(
                    mandate_id = %mandate.mandate_id,
                    error = %e,
                    "mandate not usable"
                );
                return Decision::Block {
                    reason: match &e {
                        mandate_engine::MandateError::Revoked => BlockReason::Revoked,
                        _ => BlockReason::Expired,
                    },
                    detail: e.to_string(),
                };
            }
        }

        // 3. Expiry / revocation check
        if let Err(v) = rules::check_expiry(mandate) {
            let (reason, detail) = v.into_block();
            tracing::warn!(
                mandate_id = %mandate.mandate_id,
                reason = ?reason,
                "mandate check failed"
            );
            return Decision::Block { reason, detail };
        }

        // 4. Exhaustion check
        if let Err(v) = rules::check_exhaustion(mandate) {
            let (reason, detail) = v.into_block();
            tracing::warn!(
                mandate_id = %mandate.mandate_id,
                reason = ?reason,
                "mandate exhausted"
            );
            return Decision::Block { reason, detail };
        }

        // 5. Budget check
        if let Err(v) = rules::check_budget(mandate, amount) {
            let (reason, detail) = v.into_block();
            tracing::warn!(
                mandate_id = %mandate.mandate_id,
                requested = amount,
                "budget check failed"
            );
            return Decision::Block { reason, detail };
        }

        // 6. Scope check
        if let Err(v) = rules::check_scope(mandate, category) {
            let (reason, detail) = v.into_block();
            tracing::warn!(
                mandate_id = %mandate.mandate_id,
                category = category,
                "scope check failed"
            );
            return Decision::Block { reason, detail };
        }

        tracing::info!(
            mandate_id = %mandate.mandate_id,
            amount = amount,
            category = category,
            "purchase allowed"
        );

        Decision::Allow {
            mandate_id: mandate.mandate_id.to_string(),
            amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use mandate_engine::{Frequency, Mandate, MandateSigner, NewMandate};

    #[derive(Debug)]
    struct FakeNonceChecker {
        used_nonces: std::collections::HashSet<String>,
    }

    impl FakeNonceChecker {
        fn new() -> Self {
            Self {
                used_nonces: std::collections::HashSet::new(),
            }
        }

        fn mark_used(&mut self, nonce: &str) {
            self.used_nonces.insert(nonce.to_string());
        }
    }

    impl crate::nonce_checker::NonceChecker for FakeNonceChecker {
        fn is_nonce_fresh(&self, nonce: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
            Ok(!self.used_nonces.contains(nonce))
        }
    }

    fn setup() -> (MandateSigner, Mandate, FakeNonceChecker) {
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
        let checker = FakeNonceChecker::new();
        (signer, mandate, checker)
    }

    #[test]
    fn allow_within_budget_and_scope() {
        let (signer, mandate, checker) = setup();
        let evaluator = PolicyEvaluator::new(&signer, &checker);
        let decision = evaluator.evaluate(&mandate, 30_000, "electronics");
        assert!(matches!(decision, Decision::Allow { .. }));
    }

    #[test]
    fn block_over_budget() {
        let (signer, mandate, checker) = setup();
        let evaluator = PolicyEvaluator::new(&signer, &checker);
        let decision = evaluator.evaluate(&mandate, 60_000, "electronics");
        assert!(matches!(decision, Decision::Block { reason: BlockReason::OverBudget, .. }));
    }

    #[test]
    fn block_out_of_scope() {
        let (signer, mandate, checker) = setup();
        let evaluator = PolicyEvaluator::new(&signer, &checker);
        let decision = evaluator.evaluate(&mandate, 10_000, "groceries");
        assert!(matches!(decision, Decision::Block { reason: BlockReason::OutOfScope, .. }));
    }

    #[test]
    fn block_expired_mandate() {
        let (signer, _) = MandateSigner::generate();
        let mut mandate = Mandate::new(NewMandate {
            user_id: "user-1".into(),
            merchant_id: "merchant-1".into(),
            buyer_agent_id: "agent-1".into(),
            max_amount: 50_000,
            currency: "INR".into(),
            scope: vec!["electronics".into()],
            frequency: Frequency::OneTime,
            expires_at: Utc::now() - chrono::Duration::hours(1),
        });
        signer.sign(&mut mandate).unwrap();
        let checker = FakeNonceChecker::new();

        let evaluator = PolicyEvaluator::new(&signer, &checker);
        let decision = evaluator.evaluate(&mandate, 10_000, "electronics");
        assert!(matches!(decision, Decision::Block { reason: BlockReason::Expired, .. }));
    }

    #[test]
    fn block_tampered_mandate() {
        let (signer, _) = MandateSigner::generate();
        let (signer_other, _) = MandateSigner::generate();
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
        mandate.max_amount = 999_999; // tamper
        let checker = FakeNonceChecker::new();

        let evaluator = PolicyEvaluator::new(&signer_other, &checker);
        let decision = evaluator.evaluate(&mandate, 10_000, "electronics");
        assert!(matches!(decision, Decision::Block { reason: BlockReason::ReplayDetected, .. }));
    }

    #[test]
    fn block_nonce_replay() {
        let (signer, mandate, mut checker) = setup();
        checker.mark_used(&mandate.nonce); // mark nonce as already used

        let evaluator = PolicyEvaluator::new(&signer, &checker);
        let decision = evaluator.evaluate(&mandate, 10_000, "electronics");
        assert!(matches!(decision, Decision::Block { reason: BlockReason::ReplayDetected, .. }));
    }
}
