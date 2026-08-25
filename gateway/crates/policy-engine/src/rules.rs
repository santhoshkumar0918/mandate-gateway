use mandate_engine::Mandate;

use crate::decision::BlockReason;

/// A single policy rule violation.
///
/// Each variant maps to a specific constraint check on the mandate.
/// The evaluator collects these and converts them into [`Decision`](crate::Decision)s.
#[derive(Debug, Clone)]
pub enum RuleViolation {
    OverBudget { requested: i64, remaining: i64 },
    OutOfScope { category: String, allowed: Vec<String> },
    Expired { expires_at: String },
    Revoked,
    Exhausted { spent: i64, max: i64 },
}

impl RuleViolation {
    /// Converts a violation into a block reason and human-readable detail.
    pub fn into_block(self) -> (BlockReason, String) {
        match self {
            Self::OverBudget { requested, remaining } => (
                BlockReason::OverBudget,
                format!(
                    "requested {requested} exceeds remaining budget of {remaining}"
                ),
            ),
            Self::OutOfScope { category, allowed } => (
                BlockReason::OutOfScope,
                format!(
                    "category '{category}' not in allowed scope: {allowed:?}"
                ),
            ),
            Self::Expired { expires_at } => (
                BlockReason::Expired,
                format!("mandate expired at {expires_at}"),
            ),
            Self::Revoked => (
                BlockReason::Revoked,
                "mandate has been revoked by the user".into(),
            ),
            Self::Exhausted { spent, max } => (
                BlockReason::Exhausted,
                format!("mandate budget exhausted: spent {spent} of {max}"),
            ),
        }
    }
}

/// Checks a purchase request against a mandate's constraints.
///
/// Returns `Ok(())` if all checks pass, or the first violation encountered.
/// The evaluator calls these in order — fail-fast, no partial passes.
pub fn check_budget(mandate: &Mandate, amount: i64) -> Result<(), RuleViolation> {
    let remaining = mandate.max_amount - mandate.spent_amount;
    if amount > remaining {
        return Err(RuleViolation::OverBudget {
            requested: amount,
            remaining,
        });
    }
    Ok(())
}

pub fn check_scope(mandate: &Mandate, category: &str) -> Result<(), RuleViolation> {
    if !mandate.scope.iter().any(|s| s == category) {
        return Err(RuleViolation::OutOfScope {
            category: category.into(),
            allowed: mandate.scope.clone(),
        });
    }
    Ok(())
}

pub fn check_expiry(mandate: &Mandate) -> Result<(), RuleViolation> {
    if !mandate.is_usable() {
        if mandate.status == mandate_engine::MandateStatus::Revoked {
            return Err(RuleViolation::Revoked);
        }
        return Err(RuleViolation::Expired {
            expires_at: mandate.expires_at.to_rfc3339(),
        });
    }
    Ok(())
}

pub fn check_exhaustion(mandate: &Mandate) -> Result<(), RuleViolation> {
    if mandate.spent_amount >= mandate.max_amount {
        return Err(RuleViolation::Exhausted {
            spent: mandate.spent_amount,
            max: mandate.max_amount,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use mandate_engine::{Frequency, Mandate, NewMandate};

    fn active_mandate(max_amount: i64, scope: Vec<&str>) -> Mandate {
        let mut m = Mandate::new(NewMandate {
            user_id: "user-1".into(),
            merchant_id: "merchant-1".into(),
            buyer_agent_id: "agent-1".into(),
            max_amount,
            currency: "INR".into(),
            scope: scope.into_iter().map(String::from).collect(),
            frequency: Frequency::OneTime,
            expires_at: Utc::now() + chrono::Duration::hours(1),
        });
        m.spent_amount = 0;
        m
    }

    #[test]
    fn budget_check_passes_within_limit() {
        let m = active_mandate(50_000, vec!["electronics"]);
        assert!(check_budget(&m, 40_000).is_ok());
    }

    #[test]
    fn budget_check_fails_over_limit() {
        let m = active_mandate(50_000, vec!["electronics"]);
        let err = check_budget(&m, 60_000).unwrap_err();
        assert!(matches!(err, RuleViolation::OverBudget { .. }));
    }

    #[test]
    fn scope_check_passes() {
        let m = active_mandate(50_000, vec!["electronics", "books"]);
        assert!(check_scope(&m, "electronics").is_ok());
    }

    #[test]
    fn scope_check_fails() {
        let m = active_mandate(50_000, vec!["electronics"]);
        let err = check_scope(&m, "groceries").unwrap_err();
        assert!(matches!(err, RuleViolation::OutOfScope { .. }));
    }

    #[test]
    fn expiry_check_fails_expired() {
        let mut m = active_mandate(50_000, vec!["electronics"]);
        m.expires_at = Utc::now() - chrono::Duration::hours(1);
        let err = check_expiry(&m).unwrap_err();
        assert!(matches!(err, RuleViolation::Expired { .. }));
    }

    #[test]
    fn exhaustion_check_fails() {
        let mut m = active_mandate(50_000, vec!["electronics"]);
        m.spent_amount = 50_000;
        let err = check_exhaustion(&m).unwrap_err();
        assert!(matches!(err, RuleViolation::Exhausted { .. }));
    }
}
