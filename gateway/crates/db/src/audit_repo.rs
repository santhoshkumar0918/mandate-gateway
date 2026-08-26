use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

/// Append-only audit log writer.
///
/// The audit log is immutable — there are no update or delete methods.
/// Every money-moving action writes an entry here before it's considered
/// complete. This is the "explainable" guarantee.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `params` - Audit log entry parameters
pub async fn append(pool: &PgPool, params: &AuditParams<'_>) -> Result<(), DbError> {
    sqlx::query(
        r#"INSERT INTO audit_log
            (event_type, mandate_id, entity_id, decision, reason, detail, actor)
        VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(params.event_type)
    .bind(params.mandate_id)
    .bind(params.entity_id)
    .bind(params.decision)
    .bind(params.reason)
    .bind(params.detail.clone())
    .bind(params.actor)
    .execute(pool)
    .await?;

    Ok(())
}

/// Parameters for an audit log entry.
#[derive(Debug)]
pub struct AuditParams<'a> {
    pub event_type: &'a str,
    pub mandate_id: Option<Uuid>,
    pub entity_id: &'a str,
    pub decision: &'a str,
    pub reason: Option<&'a str>,
    pub detail: Option<serde_json::Value>,
    pub actor: &'a str,
}

/// A single audit log entry, returned by queries.
#[derive(Debug, sqlx::FromRow)]
pub struct AuditEntry {
    pub id: i64,
    pub event_type: String,
    pub mandate_id: Option<Uuid>,
    pub entity_id: String,
    pub decision: String,
    pub reason: Option<String>,
    pub detail: Option<serde_json::Value>,
    pub actor: String,
    pub created_at: DateTime<Utc>,
}

/// Fetches all audit entries for a mandate, ordered by time.
pub async fn find_by_mandate(
    pool: &PgPool,
    mandate_id: Uuid,
) -> Result<Vec<AuditEntry>, DbError> {
    let rows = sqlx::query_as::<_, AuditEntry>(
        "SELECT * FROM audit_log WHERE mandate_id = $1 ORDER BY created_at ASC",
    )
    .bind(mandate_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
