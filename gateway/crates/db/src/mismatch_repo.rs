use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

pub async fn insert(
    pool: &PgPool,
    mismatch_id: Uuid,
    intent_id: Option<Uuid>,
    mandate_id: Uuid,
    kind: &serde_json::Value,
    status: &str,
) -> Result<(), DbError> {
    sqlx::query(
        r#"INSERT INTO mismatches (mismatch_id, intent_id, mandate_id, kind, status)
        VALUES ($1, $2, $3, $4, $5)"#,
    )
    .bind(mismatch_id)
    .bind(intent_id)
    .bind(mandate_id)
    .bind(kind)
    .bind(status)
    .execute(pool)
    .await?;

    Ok(())
}

/// Updates a mismatch's status and optionally sets the refund_id.
///
/// Used in the reconciliation lifecycle:
/// detected → refund_initiated (set refund_id) → refund_completed
pub async fn update_status(
    pool: &PgPool,
    mismatch_id: Uuid,
    status: &str,
    refund_id: Option<&str>,
    refund_initiated_at: Option<DateTime<Utc>>,
    refund_completed_at: Option<DateTime<Utc>>,
) -> Result<(), DbError> {
    sqlx::query(
        r#"UPDATE mismatches
           SET status = $1,
               refund_id = COALESCE($2, refund_id),
               refund_initiated_at = COALESCE($3, refund_initiated_at),
               refund_completed_at = COALESCE($4, refund_completed_at)
           WHERE mismatch_id = $5"#,
    )
    .bind(status)
    .bind(refund_id)
    .bind(refund_initiated_at)
    .bind(refund_completed_at)
    .bind(mismatch_id)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct MismatchRow {
    pub mismatch_id: Uuid,
    pub intent_id: Option<Uuid>,
    pub mandate_id: Uuid,
    pub kind: serde_json::Value,
    pub status: String,
    pub refund_id: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub refund_initiated_at: Option<DateTime<Utc>>,
    pub refund_completed_at: Option<DateTime<Utc>>,
}

pub async fn find_by_mandate(
    pool: &PgPool,
    mandate_id: Uuid,
) -> Result<Vec<MismatchRow>, DbError> {
    let rows = sqlx::query_as::<_, MismatchRow>(
        "SELECT * FROM mismatches WHERE mandate_id = $1 ORDER BY detected_at ASC",
    )
    .bind(mandate_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn find_by_id(
    pool: &PgPool,
    mismatch_id: Uuid,
) -> Result<Option<MismatchRow>, DbError> {
    let row = sqlx::query_as::<_, MismatchRow>(
        "SELECT * FROM mismatches WHERE mismatch_id = $1",
    )
    .bind(mismatch_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Recent mismatches, newest first (reconciliation operator view).
pub async fn list_recent(pool: &PgPool, limit: i64) -> Result<Vec<MismatchRow>, DbError> {
    let rows = sqlx::query_as::<_, MismatchRow>(
        "SELECT * FROM mismatches ORDER BY detected_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
