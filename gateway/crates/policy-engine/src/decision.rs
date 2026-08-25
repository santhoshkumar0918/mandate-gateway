use serde::{Deserialize, Serialize};

/// The outcome of evaluating a purchase request against a mandate.
///
/// Every purchase request produces exactly one `Decision`. The caller must
/// respect the decision — an `Allow` is authorization to proceed, a `Block`
/// is a hard stop, and `Escalate` means human review is required.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Decision {
    /// Purchase is within mandate bounds. Proceed.
    Allow {
        /// The mandate that authorized this decision.
        mandate_id: String,
        /// Amount being authorized (currency subunits).
        amount: i64,
    },
    /// Mandate violation detected. Hard stop — do not proceed.
    Block {
        reason: BlockReason,
        /// Human-readable explanation for audit log.
        detail: String,
    },
    /// Ambiguous case — needs human review before proceeding.
    Escalate {
        reason: String,
        /// The specific rule that triggered escalation.
        rule: String,
    },
}

/// Why a purchase was blocked.
///
/// Each variant maps to a specific mandate constraint that was violated.
/// This makes the audit log queryable by block type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlockReason {
    /// Purchase amount exceeds mandate's max_amount.
    OverBudget,
    /// Requested category/SKU is not in the mandate's scope.
    OutOfScope,
    /// Mandate has expired.
    Expired,
    /// Mandate has been revoked by the user.
    Revoked,
    /// Mandate budget fully consumed.
    Exhausted,
    /// Duplicate nonce detected — possible replay attack.
    ReplayDetected,
}

impl Decision {
    /// Returns the decision kind as a string for audit logging.
    pub fn kind(&self) -> DecisionKind {
        match self {
            Decision::Allow { .. } => DecisionKind::Allow,
            Decision::Block { .. } => DecisionKind::Block,
            Decision::Escalate { .. } => DecisionKind::Escalate,
        }
    }
}

/// String tag for decision type — used in audit log queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Allow,
    Block,
    Escalate,
}
