use crate::types::{Intent, Mismatch, MismatchKind, Outcome};

/// Detects mismatches between buyer agent intent and actual purchase outcome.
///
/// Compares the intent (captured at mandate request time) against the outcome
/// (captured from Razorpay's Order/Payment response) and produces [`Mismatch`]
/// records for any divergences.
///
/// # Invariant
///
/// This is deterministic — same inputs always produce the same mismatches.
/// No randomness, no external calls, no state. Pure comparison logic.
pub struct MismatchDetector;

impl MismatchDetector {
    /// Compares an intent against an outcome and returns all detected mismatches.
    ///
    /// Returns an empty `Vec` if intent and outcome match exactly. Returns
    /// one or more [`Mismatch`] records for each divergence found.
    pub fn detect(intent: &Intent, outcome: &Outcome) -> Vec<Mismatch> {
        let mut mismatches = Vec::new();

        // Price drift — the core engineered failure scenario
        if intent.expected_price != outcome.actual_price {
            mismatches.push(Mismatch::new(
                Some(intent.intent_id),
                intent.mandate_id,
                MismatchKind::PriceDrift {
                    expected: intent.expected_price,
                    actual: outcome.actual_price,
                },
            ));
        }

        // Wrong product
        if intent.product_id != outcome.product_id {
            mismatches.push(Mismatch::new(
                Some(intent.intent_id),
                intent.mandate_id,
                MismatchKind::WrongProduct {
                    expected: intent.product_id.clone(),
                    actual: outcome.product_id.clone(),
                },
            ));
        }

        mismatches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn test_intent(expected_price: i64) -> Intent {
        Intent {
            intent_id: Uuid::new_v4(),
            mandate_id: Uuid::new_v4(),
            product_id: "prod-001".into(),
            category: "electronics".into(),
            expected_price,
            currency: "INR".into(),
            created_at: Utc::now(),
        }
    }

    fn test_outcome(actual_price: i64, product_id: &str) -> Outcome {
        Outcome {
            order_id: "order-001".into(),
            payment_id: "pay-001".into(),
            product_id: product_id.into(),
            actual_price,
            currency: "INR".into(),
            completed_at: Utc::now(),
        }
    }

    #[test]
    fn no_mismatch_when_match() {
        let intent = test_intent(100);
        let outcome = test_outcome(100, "prod-001");
        let mismatches = MismatchDetector::detect(&intent, &outcome);
        assert!(mismatches.is_empty());
    }

    #[test]
    fn price_drift_detected() {
        let intent = test_intent(100);
        let outcome = test_outcome(150, "prod-001");
        let mismatches = MismatchDetector::detect(&intent, &outcome);
        assert_eq!(mismatches.len(), 1);
        assert!(matches!(
            mismatches[0].kind,
            MismatchKind::PriceDrift { expected: 100, actual: 150 }
        ));
    }

    #[test]
    fn wrong_product_detected() {
        let intent = test_intent(100);
        let outcome = test_outcome(100, "prod-002");
        let mismatches = MismatchDetector::detect(&intent, &outcome);
        assert_eq!(mismatches.len(), 1);
        assert!(matches!(
            mismatches[0].kind,
            MismatchKind::WrongProduct { .. }
        ));
    }

    #[test]
    fn multiple_mismatches_detected() {
        let intent = test_intent(100);
        let outcome = test_outcome(200, "prod-002");
        let mismatches = MismatchDetector::detect(&intent, &outcome);
        assert_eq!(mismatches.len(), 2);
    }
}
