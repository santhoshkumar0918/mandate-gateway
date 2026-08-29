use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What the buyer agent intended to purchase.
///
/// Captured at mandate request time — represents the agent's original intent
/// before any price drift or inventory changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub intent_id: Uuid,
    pub mandate_id: Uuid,
    pub product_id: String,
    pub category: String,
    /// The price the agent expected at intent time (currency subunits).
    pub expected_price: i64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

/// What actually happened after the purchase.
///
/// Captured from the Razorpay Order/Payment response — represents the
/// actual outcome, which may differ from the intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub order_id: String,
    pub payment_id: String,
    pub product_id: String,
    /// The price actually charged (currency subunits).
    pub actual_price: i64,
    pub currency: String,
    pub completed_at: DateTime<Utc>,
}

/// A detected mismatch between intent and outcome.
///
/// The detector compares Intent and Outcome and produces one or more of these.
/// Each mismatch has a kind (what diverged), a status (lifecycle), and
/// timestamps for audit trail completeness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mismatch {
    pub mismatch_id: Uuid,
    /// The buyer intent this mismatch relates to. `None` for mismatch kinds
    /// that have no buyer intent (e.g. fulfillment timeout on a merchant
    /// order), which is why the column is nullable.
    pub intent_id: Option<Uuid>,
    pub mandate_id: Uuid,
    pub kind: MismatchKind,
    pub status: MismatchStatus,
    pub detected_at: DateTime<Utc>,
    /// When the refund was initiated (if applicable).
    pub refund_initiated_at: Option<DateTime<Utc>>,
    /// When the refund completed (if applicable).
    pub refund_completed_at: Option<DateTime<Utc>>,
    /// Razorpay refund ID (if applicable).
    pub refund_id: Option<String>,
}

/// What diverged between intent and outcome.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MismatchKind {
    /// Price drifted — agent expected X but was charged Y.
    PriceDrift {
        expected: i64,
        actual: i64,
    },
    /// Wrong product delivered — intent product_id doesn't match outcome.
    WrongProduct {
        expected: String,
        actual: String,
    },
    /// Category mismatch — intent category doesn't match outcome.
    CategoryMismatch {
        expected: String,
        actual: String,
    },
    /// Fulfillment timeout — an order was created but no proof of delivery
    /// arrived within the SLA, so the spend is recovered (verify-then-pay).
    FulfillmentTimeout {
        order_id: String,
    },
}

/// Lifecycle status of a mismatch.
///
/// Transitions: Detected → RefundInitiated → RefundCompleted.
/// A second mismatch arriving while status is RefundInitiated is
/// serialized via row-level locking, not queued separately.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MismatchStatus {
    Detected,
    RefundInitiated,
    RefundCompleted,
}

impl Mismatch {
    /// Creates a new mismatch in Detected status.
    pub fn new(intent_id: Option<Uuid>, mandate_id: Uuid, kind: MismatchKind) -> Self {
        Self {
            mismatch_id: Uuid::new_v4(),
            intent_id,
            mandate_id,
            kind,
            status: MismatchStatus::Detected,
            detected_at: Utc::now(),
            refund_initiated_at: None,
            refund_completed_at: None,
            refund_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_mismatch_is_detected() {
        let m = Mismatch::new(
            Some(Uuid::new_v4()),
            Uuid::new_v4(),
            MismatchKind::PriceDrift {
                expected: 100,
                actual: 150,
            },
        );
        assert_eq!(m.status, MismatchStatus::Detected);
        assert!(m.refund_initiated_at.is_none());
    }
}
