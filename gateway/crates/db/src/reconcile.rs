use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

/// Serializes concurrent reconciliation attempts on a single mandate and
/// reports whether a refund has already been initiated for the given order.
///
/// Holds a `SELECT ... FOR UPDATE` row lock on the mandate for the duration
/// of the check, so two concurrent reconciliations of the same mandate
/// serialize at the database rather than racing. Returns `Ok(true)` if a
/// refund already exists for this order (idempotency short-circuit), or
/// `Ok(false)` if recovery should proceed.
///
/// The lock is only held for this brief check-and-decision, not across the
/// external refund call.
pub async fn lock_mandate_and_check_refund_exists(
    pool: &PgPool,
    mandate_id: Uuid,
    order_id: &str,
) -> Result<bool, DbError> {
    let mut tx = pool.begin().await?;

    let locked = sqlx::query(
        "SELECT mandate_id FROM mandates WHERE mandate_id = $1 FOR UPDATE",
    )
    .bind(mandate_id)
    .fetch_optional(&mut *tx)
    .await?;

    if locked.is_none() {
        return Err(DbError::NotFound(format!("mandate {} not found", mandate_id)));
    }

    let idempotency_key = format!("refund:{}", order_id);
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM refunds WHERE idempotency_key = $1",
    )
    .bind(&idempotency_key)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(exists > 0)
}

/// Parameters for recording a refund recovery in one transaction.
pub struct RecordRecoveryParams<'a> {
    pub mismatch_id: Uuid,
    pub refund_id: &'a str,
    pub order_id: &'a str,
    pub payment_id: &'a str,
    pub mandate_id: Uuid,
    pub amount: i64,
    pub currency: &'a str,
    pub status: &'a str,
    /// True if the refund API reported completion, false if still pending.
    pub completed: bool,
    pub event: &'a str,
    pub actor: &'a str,
}

/// Records the recovery state for a resolved mismatch inside one transaction.
///
/// Updates the mismatch to `refund_completed` (or `refund_initiated`),
/// inserts the refund row with an idempotency key derived from the order,
/// and appends an audit event — all in the same transaction, so the refunded
/// money record and its audit trail cannot diverge on a crash.
///
/// `completed` marks whether the refund API reported completion; when false
/// the lifecycle stays at `refund_initiated` awaiting confirmation.
pub async fn record_recovery(
    pool: &PgPool,
    p: &RecordRecoveryParams<'_>,
) -> Result<(), DbError> {
    let mut tx = pool.begin().await?;

    let init_at = if p.completed { None } else { Some(Utc::now()) };
    let comp_at = if p.completed { Some(Utc::now()) } else { None };

    let mismatch_status = if p.completed { "refund_completed" } else { "refund_initiated" };

    let idempotency_key = format!("refund:{}", p.order_id);

    // Insert the refund row first so the mismatch's refund_id FK resolves.
    sqlx::query(
        r#"INSERT INTO refunds
            (refund_id, payment_id, mandate_id, amount, currency, status, idempotency_key)
        VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(p.refund_id)
    .bind(p.payment_id)
    .bind(p.mandate_id)
    .bind(p.amount)
    .bind(p.currency)
    .bind(p.status)
    .bind(&idempotency_key)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE mismatches
           SET status = $1,
               refund_id = $2,
               refund_initiated_at = COALESCE($3, refund_initiated_at),
               refund_completed_at = COALESCE($4, refund_completed_at)
         WHERE mismatch_id = $5",
    )
    .bind(mismatch_status)
    .bind(Some(p.refund_id))
    .bind(init_at)
    .bind(comp_at)
    .bind(p.mismatch_id)
    .execute(&mut *tx)
    .await?;

    crate::audit_repo::append_tx(
        &mut tx,
        &crate::audit_repo::AuditParams {
            event_type: p.event,
            mandate_id: Some(p.mandate_id),
            entity_id: p.refund_id,
            decision: "allow",
            reason: None,
            detail: Some(serde_json::json!({
                "refund_id": p.refund_id,
                "amount": p.amount,
                "status": p.status,
                "idempotency_key": idempotency_key,
            })),
            actor: p.actor,
        },
    )
    .await?;

    tx.commit().await?;

    Ok(())
}
