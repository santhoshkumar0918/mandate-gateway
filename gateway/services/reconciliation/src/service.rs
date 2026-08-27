use db::PgPool;

use crate::error::ReconciliationError;
use crate::matcher::MismatchDetector;
use crate::refund_provider::RefundProvider;
use crate::types::{Intent, Mismatch, MismatchKind, Outcome};

/// Orchestrates intent-vs-outcome reconciliation and recovery.
///
/// Owns the only code path that triggers a refund. Detects mismatches
/// between a buyer agent's intent and the actual purchase outcome, persists
/// them, and drives them through a refund lifecycle:
/// `detected → refund_initiated → refund_completed`.
///
/// # Money invariants
///
/// - No refund is ever issued without first writing the detected mismatch
///   and holding the per-mandate row lock (see [`db::reconcile`]).
/// - Every refund call carries an idempotency key derived from the order, so
///   a retried trigger can never double-refund.
/// - Every money action and its audit entry commit in the same transaction.
#[derive(Clone)]
pub struct ReconcileService<P: RefundProvider + Clone> {
    db: PgPool,
    refunds: P,
}

impl<P: RefundProvider + Clone> ReconcileService<P> {
    pub fn new(db: PgPool, refunds: P) -> Self {
        Self { db, refunds }
    }

    /// Compares an intent against the actual outcome and persists any
    /// mismatches, then triggers recovery (refund) for each.
    ///
    /// Returns the set of detected mismatches. When intent and outcome
    /// match, nothing is persisted beyond an audit confirmation.
    pub async fn reconcile_purchase(
        &self,
        intent: &Intent,
        outcome: &Outcome,
        actor: &str,
    ) -> Result<Vec<Mismatch>, ReconciliationError> {
        let detected = MismatchDetector::detect(intent, outcome);

        if detected.is_empty() {
            db::audit_repo::append(
                &self.db,
                &db::audit_repo::AuditParams {
                    event_type: "order_reconciled",
                    mandate_id: Some(intent.mandate_id),
                    entity_id: &outcome.order_id,
                    decision: "allow",
                    reason: None,
                    detail: Some(serde_json::json!({
                        "amount": outcome.actual_price,
                        "product_id": outcome.product_id,
                    })),
                    actor,
                },
            )
            .await?;
            return Ok(detected);
        }

        for m in &detected {
            self.persist_detected(m, actor).await?;
            self.trigger_refund(outcome, m, actor).await?;
        }

        Ok(detected)
    }

    /// Persists a detected mismatch in `Detected` status.
    async fn persist_detected(&self, m: &Mismatch, actor: &str) -> Result<(), ReconciliationError> {
        let kind_json = serde_json::to_value(&m.kind)?;
        db::mismatch_repo::insert(
            &self.db,
            m.mismatch_id,
            m.intent_id,
            m.mandate_id,
            &kind_json,
            "detected",
        )
        .await?;

        db::audit_repo::append(
            &self.db,
            &db::audit_repo::AuditParams {
                event_type: "mismatch_detected",
                mandate_id: Some(m.mandate_id),
                entity_id: &m.mismatch_id.to_string(),
                decision: "block",
                reason: Some(&describe_kind(&m.kind)),
                detail: Some(serde_json::to_value(&m.kind)?),
                actor,
            },
        )
        .await?;

        Ok(())
    }

    /// Attempts recovery for a single mismatch and records the refund.
    ///
    /// The idempotency short-circuit runs under the mandate row lock, so a
    /// concurrent reconciliation of the same order cannot double-refund.
    ///
    /// A refund API failure is audited as `refund_failed` and logged, but is
    /// not fatal to the caller — the purchase itself already succeeded, and a
    /// failed refund attempt must not roll back a completed money movement.
    async fn trigger_refund(
        &self,
        outcome: &Outcome,
        m: &Mismatch,
        actor: &str,
    ) -> Result<(), ReconciliationError> {
        let already_refunded = db::reconcile::lock_mandate_and_check_refund_exists(
            &self.db,
            m.mandate_id,
            &outcome.order_id,
        )
        .await?;

        if already_refunded {
            tracing::info!(
                order_id = %outcome.order_id,
                "reconciliation: refund already exists for order, skipping"
            );
            return Ok(());
        }

        let refund = match self
            .refunds
            .issue_refund(&outcome.payment_id, Some(outcome.actual_price))
            .await
        {
            Ok(refund) => refund,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    order_id = %outcome.order_id,
                    "reconciliation: refund request failed"
                );
                db::audit_repo::append(
                    &self.db,
                    &db::audit_repo::AuditParams {
                        event_type: "refund_failed",
                        mandate_id: Some(m.mandate_id),
                        entity_id: &outcome.order_id,
                        decision: "block",
                        reason: Some("refund request rejected by Razorpay"),
                        detail: Some(serde_json::json!({
                            "error": e.to_string(),
                            "amount": outcome.actual_price,
                        })),
                        actor,
                    },
                )
                .await?;
                return Ok(());
            }
        };

        db::reconcile::record_recovery(
            &self.db,
            &db::reconcile::RecordRecoveryParams {
                mismatch_id: m.mismatch_id,
                refund_id: &refund.id,
                order_id: &outcome.order_id,
                payment_id: &outcome.payment_id,
                mandate_id: m.mandate_id,
                amount: outcome.actual_price,
                currency: &outcome.currency,
                status: &refund.status,
                completed: true,
                event: "refund_triggered",
                actor,
            },
        )
        .await?;

        Ok(())
    }
}

fn describe_kind(kind: &MismatchKind) -> String {
    match kind {
        MismatchKind::PriceDrift { expected, actual } => {
            format!("price drift: expected {} but charged {}", expected, actual)
        }
        MismatchKind::WrongProduct { expected, actual } => {
            format!("wrong product: expected {} but got {}", expected, actual)
        }
        MismatchKind::CategoryMismatch { expected, actual } => {
            format!("category mismatch: expected {} but got {}", expected, actual)
        }
    }
}
