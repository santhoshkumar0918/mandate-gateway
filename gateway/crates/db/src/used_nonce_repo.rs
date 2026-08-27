use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::DbError;

/// Records a per-purchase authorization nonce as consumed, atomically.
///
/// The UNIQUE constraint on `used_nonces.nonce` is the trust-critical
/// replay guard: attempting to consume the same nonce twice fails on the
/// second insert. Returns `true` if the nonce was newly consumed, `false`
/// if it was already present (a replay).
pub async fn mark_used(
    txn: &mut Transaction<'_, Postgres>,
    mandate_id: Uuid,
    nonce: &str,
) -> Result<bool, DbError> {
    let result = sqlx::query(
        "INSERT INTO used_nonces (nonce, mandate_id) VALUES ($1, $2) ON CONFLICT (nonce) DO NOTHING",
    )
    .bind(nonce)
    .bind(mandate_id)
    .execute(&mut **txn)
    .await?;

    Ok(result.rows_affected() == 1)
}

/// Checks whether a nonce was already consumed (replay detection helper).
///
/// Mainly useful for tests and diagnostics; the authoritative replay guard is
/// the atomic `mark_used` insert within the spend transaction.
pub async fn is_used(pool: &PgPool, nonce: &str) -> Result<bool, DbError> {
    let result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM used_nonces WHERE nonce = $1")
        .bind(nonce)
        .fetch_one(pool)
        .await?;
    Ok(result > 0)
}
