use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use mandate_engine::{Frequency, Mandate, MandateStatus};

use crate::error::DbError;

/// Inserts a signed mandate into the mandates table.
pub async fn insert(pool: &PgPool, mandate: &Mandate) -> Result<(), DbError> {
    let scope_json =
        serde_json::to_value(&mandate.scope).map_err(|e| DbError::Serialization(e.to_string()))?;

    let frequency_str = match mandate.frequency {
        Frequency::OneTime => "one_time",
        Frequency::Recurring => "recurring",
    };

    let status_str = match mandate.status {
        MandateStatus::Active => "active",
        MandateStatus::Revoked => "revoked",
        MandateStatus::Expired => "expired",
        MandateStatus::Exhausted => "exhausted",
    };

    sqlx::query(
        r#"INSERT INTO mandates
            (mandate_id, issued_at, expires_at, user_id, merchant_id,
             buyer_agent_id, max_amount, currency, scope, frequency,
             spent_amount, status, nonce, signature)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"#,
    )
    .bind(mandate.mandate_id)
    .bind(mandate.issued_at)
    .bind(mandate.expires_at)
    .bind(&mandate.user_id)
    .bind(&mandate.merchant_id)
    .bind(&mandate.buyer_agent_id)
    .bind(mandate.max_amount)
    .bind(&mandate.currency)
    .bind(scope_json)
    .bind(frequency_str)
    .bind(mandate.spent_amount)
    .bind(status_str)
    .bind(&mandate.nonce)
    .bind(&mandate.signature)
    .execute(pool)
    .await?;

    Ok(())
}

/// Finds a mandate by its UUID.
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Mandate>, DbError> {
    let row = sqlx::query_as::<_, MandateRow>(
        "SELECT * FROM mandates WHERE mandate_id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into()))
}

/// Finds a mandate by nonce — used for replay protection.
pub async fn find_by_nonce(pool: &PgPool, nonce: &str) -> Result<Option<Mandate>, DbError> {
    let row = sqlx::query_as::<_, MandateRow>(
        "SELECT * FROM mandates WHERE nonce = $1",
    )
    .bind(nonce)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into()))
}

/// Atomically increments the spent_amount on a mandate.
///
/// This is the double-spend prevention. Two concurrent calls will serialize
/// at the row level — the second one sees the updated spent_amount from the
/// first. Returns Err if the increment would exceed max_amount.
pub async fn increment_spent(
    pool: &PgPool,
    id: Uuid,
    amount: i64,
) -> Result<(), DbError> {
    let result = sqlx::query(
        r#"UPDATE mandates
           SET spent_amount = spent_amount + $1
           WHERE mandate_id = $2
             AND spent_amount + $1 <= max_amount
             AND status = 'active'"#,
    )
    .bind(amount)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!(
            "mandate {} not found, not active, or insufficient budget",
            id
        )));
    }

    Ok(())
}

/// Updates the status of a mandate (e.g., revoke, mark exhausted).
pub async fn update_status(
    pool: &PgPool,
    id: Uuid,
    status: MandateStatus,
) -> Result<(), DbError> {
    let status_str = match status {
        MandateStatus::Active => "active",
        MandateStatus::Revoked => "revoked",
        MandateStatus::Expired => "expired",
        MandateStatus::Exhausted => "exhausted",
    };

    sqlx::query("UPDATE mandates SET status = $1 WHERE mandate_id = $2")
        .bind(status_str)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Lists all mandates for a merchant.
pub async fn list_by_merchant(
    pool: &PgPool,
    merchant_id: &str,
) -> Result<Vec<Mandate>, DbError> {
    let rows = sqlx::query_as::<_, MandateRow>(
        "SELECT * FROM mandates WHERE merchant_id = $1 ORDER BY created_at DESC",
    )
    .bind(merchant_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// Checks whether a nonce has been used before — replay detection.
///
/// Returns `true` if the nonce exists in any mandate row, `false` if fresh.
pub async fn nonce_exists(pool: &PgPool, nonce: &str) -> Result<bool, DbError> {
    let result = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM mandates WHERE nonce = $1",
    )
    .bind(nonce)
    .fetch_one(pool)
    .await?;

    Ok(result > 0)
}

/// Raw row type that maps to the Postgres mandates table.
/// We convert to the domain `Mandate` type at the boundary.
#[derive(sqlx::FromRow)]
struct MandateRow {
    mandate_id: Uuid,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    user_id: String,
    merchant_id: String,
    buyer_agent_id: String,
    max_amount: i64,
    currency: String,
    scope: serde_json::Value,
    frequency: String,
    spent_amount: i64,
    status: String,
    nonce: String,
    signature: Vec<u8>,
}

impl From<MandateRow> for Mandate {
    fn from(row: MandateRow) -> Self {
        let scope: Vec<String> = serde_json::from_value(row.scope).unwrap_or_default();
        let frequency = match row.frequency.as_str() {
            "recurring" => Frequency::Recurring,
            _ => Frequency::OneTime,
        };
        let status = match row.status.as_str() {
            "revoked" => MandateStatus::Revoked,
            "expired" => MandateStatus::Expired,
            "exhausted" => MandateStatus::Exhausted,
            _ => MandateStatus::Active,
        };

        Mandate {
            mandate_id: row.mandate_id,
            issued_at: row.issued_at,
            expires_at: row.expires_at,
            user_id: row.user_id,
            merchant_id: row.merchant_id,
            buyer_agent_id: row.buyer_agent_id,
            max_amount: row.max_amount,
            currency: row.currency,
            scope,
            frequency,
            spent_amount: row.spent_amount,
            status,
            nonce: row.nonce,
            signature: row.signature,
        }
    }
}
