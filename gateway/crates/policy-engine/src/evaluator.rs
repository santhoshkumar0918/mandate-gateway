use mandate_engine::{Mandate, MandateSigner};

use crate::decision::{BlockReason, Decision};
use crate::rules;

/// Deterministic policy evaluator — no LLM, no ML, plain code.
///
/// Runs a purchase request through all policy rules and produces a single
/// [`Decision`]. Every decision is logged via `tracing` for the audit trail.
///
/// # Order of evaluation
///
/// 1. Signature verification (mandate integrity)
/// 2. Expiry / revocation check
/// 3. Exhaustion check
/// 4. Budget check (amount vs remaining)
/// 5. Scope check (category in whitelist)
///
/// Fail-fast: the first violation produces the decision. No partial passes.
pub struct PolicyEvaluator<'a> {
    signer: &'a MandateSigner,
}

impl<'a> PolicyEvaluator<'a> {
    pub fn new(signer: &'a MandateSigner) -> Self {
        Self { signer }
    }

    /// Evaluates a purchase request against the given mandate.
    ///
    /// Returns a [`Decision`] — Allow, Block, or Escalate. The caller must
    /// respect the decision before making any payment call.
    pub fn evaluate(&self, mandate: &Mandate, amount: i64, category: &str) -> Decision {
        // 1. Signature verification — tampered mandate = hard block
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

        // 2. Expiry / revocation check
        if let Err(v) = rules::check_expiry(mandate) {
            let (reason, detail) = v.into_block();
            tracing::warn!(
                mandate_id = %mandate.mandate_id,
                reason = ?reason,
                "mandate check failed"
            );
            return Decision::Block { reason, detail };
        }

        // 3. Exhaustion check
        if let Err(v) = rules::check_exhaustion(mandate) {
            let (reason, detail) = v.into_block();
            tracing::warn!(
                mandate_id = %mandate.mandate_id,
                reason = ?reason,
                "mandate exhausted"
            );
            return Decision::Block { reason, detail };
        }

        // 4. Budget check
        if let Err(v) = rules::check_budget(mandate, amount) {
            let (reason, detail) = v.into_block();
            tracing::warn!(
                mandate_id = %mandate.mandate_id,
                requested = amount,
                "budget check failed"
            );
            return Decision::Block { reason, detail };
        }

        // 5. Scope check
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

    fn setup() -> (MandateSigner, Mandate) {
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
        (signer, mandate)
    }

    #[test]
    fn allow_within_budget_and_scope() {
        let (signer, mandate) = setup();
        let evaluator = PolicyEvaluator::new(&signer);
        let decision = evaluator.evaluate(&mandate, 30_000, "electronics");
        assert!(matches!(decision, Decision::Allow { .. }));
    }

    #[test]
    fn block_over_budget() {
        let (signer, mandate) = setup();
        let evaluator = PolicyEvaluator::new(&signer);
        let decision = evaluator.evaluate(&mandate, 60_000, "electronics");
        assert!(matches!(decision, Decision::Block { reason: BlockReason::OverBudget, .. }));
    }

    #[test]
    fn block_out_of_scope() {
        let (signer, mandate) = setup();
        let evaluator = PolicyEvaluator::new(&signer);
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

        let evaluator = PolicyEvaluator::new(&signer);
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

        let evaluator = PolicyEvaluator::new(&signer_other);
        let decision = evaluator.evaluate(&mandate, 10_000, "electronics");
        assert!(matches!(decision, Decision::Block { reason: BlockReason::ReplayDetected, .. }));
    }
}
