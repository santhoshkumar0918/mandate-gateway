use mandate_engine::{Mandate, MandateSigner};

use crate::decision::{BlockReason, Decision};
use crate::rules;

pub struct PolicyEvaluator<'a> {
    signer: &'a MandateSigner,
}

impl<'a> PolicyEvaluator<'a> {
    pub fn new(signer: &'a MandateSigner) -> Self {
        Self { signer }
    }

    /// Evaluates a single purchase authorization against a mandate.
    ///
    /// The `amount` and `category` in scope/budget checks come from the
    /// authorization itself — which is signed — so a caller can't argue a
    /// different amount after the fact.
    ///
    /// Nonce replay is NOT checked here: the authorization's nonce is consumed
    /// atomically with the budget debit by the caller (see the `used_nonces`
    /// table), so a replayed authorization can never move money twice even
    /// under concurrency.
    pub fn evaluate(&self, mandate: &Mandate, auth: &mandate_engine::PurchaseAuth) -> Decision {
        let amount = auth.amount;
        let category = &auth.category;

        // 1. Mandate signature — proves the mandate was issued and un-tampered.
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

        // 2. Authorization signature — this specific purchase was signed.
        match self.signer.verify_auth(auth) {
            Ok(true) => {}
            Ok(false) => {
                tracing::warn!(
                    auth_id = %auth.auth_id,
                    mandate_id = %mandate.mandate_id,
                    "purchase authorization signature verification failed"
                );
                return Decision::Block {
                    reason: BlockReason::ReplayDetected,
                    detail: "purchase authorization signature verification failed".into(),
                };
            }
            Err(e) => {
                tracing::warn!(
                    auth_id = %auth.auth_id,
                    error = %e,
                    "purchase authorization not signed"
                );
                return Decision::Block {
                    reason: BlockReason::ReplayDetected,
                    detail: format!("purchase authorization not signed: {e}"),
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

        // 5. Budget check (against the signed authorization amount)
        if let Err(v) = rules::check_budget(mandate, amount) {
            let (reason, detail) = v.into_block();
            tracing::warn!(
                mandate_id = %mandate.mandate_id,
                requested = amount,
                "budget check failed"
            );
            return Decision::Block { reason, detail };
        }

        // 6. Scope check (against the signed authorization category)
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
    use mandate_engine::purchase_auth::PurchaseAuth;
    use mandate_engine::{Frequency, Mandate, MandateSigner, NewMandate};
    use uuid::Uuid;

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

    #[test]
    fn allow_within_budget_and_scope() {
        let (signer, mandate) = setup();
        let evaluator = PolicyEvaluator::new(&signer);
        let auth = signed_auth(&signer, &mandate, 30_000, "electronics");
        let decision = evaluator.evaluate(&mandate, &auth);
        assert!(matches!(decision, Decision::Allow { .. }));
    }

    #[test]
    fn block_over_budget() {
        let (signer, mandate) = setup();
        let evaluator = PolicyEvaluator::new(&signer);
        let auth = signed_auth(&signer, &mandate, 60_000, "electronics");
        let decision = evaluator.evaluate(&mandate, &auth);
        assert!(matches!(decision, Decision::Block { reason: BlockReason::OverBudget, .. }));
    }

    #[test]
    fn block_out_of_scope() {
        let (signer, mandate) = setup();
        let evaluator = PolicyEvaluator::new(&signer);
        let auth = signed_auth(&signer, &mandate, 10_000, "groceries");
        let decision = evaluator.evaluate(&mandate, &auth);
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
        let mut auth = PurchaseAuth::new(
            Uuid::new_v4(),
            10_000,
            "INR",
            "prod-001",
            "electronics",
        );
        signer.sign_auth(&mut auth).unwrap();

        let evaluator = PolicyEvaluator::new(&signer);
        let decision = evaluator.evaluate(&mandate, &auth);
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
        let mut auth = PurchaseAuth::new(
            Uuid::new_v4(),
            10_000,
            "INR",
            "prod-001",
            "electronics",
        );
        signer.sign_auth(&mut auth).unwrap();

        let evaluator = PolicyEvaluator::new(&signer_other);
        let decision = evaluator.evaluate(&mandate, &auth);
        assert!(matches!(decision, Decision::Block { reason: BlockReason::ReplayDetected, .. }));
    }

    #[test]
    fn block_unsigned_auth() {
        let (signer, mandate) = setup();
        let evaluator = PolicyEvaluator::new(&signer);
        let auth = PurchaseAuth::new(
            mandate.mandate_id,
            30_000,
            "INR",
            "prod-001",
            "electronics",
        ); // never signed
        let decision = evaluator.evaluate(&mandate, &auth);
        assert!(matches!(decision, Decision::Block { reason: BlockReason::ReplayDetected, .. }));
    }

    #[test]
    fn block_tampered_auth() {
        let (signer, mandate) = setup();
        let evaluator = PolicyEvaluator::new(&signer);
        let mut auth = signed_auth(&signer, &mandate, 30_000, "electronics");
        auth.amount = auth.amount + 1; // tamper after signing
        let decision = evaluator.evaluate(&mandate, &auth);
        assert!(matches!(decision, Decision::Block { reason: BlockReason::ReplayDetected, .. }));
    }
}
