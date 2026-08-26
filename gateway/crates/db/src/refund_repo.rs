use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

/// Parameters for inserting a refund record.
#[derive(Debug)]
pub struct InsertRefundParams<'a> {
    pub refund_id: &'a str,
    pub payment_id: &'a str,
    pub mandate_id: Uuid,
    pub amount: i64,
    pub currency: &'a str,
    pub status: &'a str,
    pub idempotency_key: &'a str,
}

pub async fn insert(pool: &PgPool, params: &InsertRefundParams<'_>) -> Result<(), DbError> {
    sqlx::query(
        r#"INSERT INTO refunds
            (refund_id, payment_id, mandate_id, amount, currency, status, idempotency_key)
        VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(params.refund_id)
    .bind(params.payment_id)
    .bind(params.mandate_id)
    .bind(params.amount)
    .bind(params.currency)
    .bind(params.status)
    .bind(params.idempotency_key)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct RefundRow {
    pub refund_id: String,
    pub payment_id: String,
    pub mandate_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
}

pub async fn find_by_idempotency_key(
    pool: &PgPool,
    key: &str,
) -> Result<Option<RefundRow>, DbError> {
    let row = sqlx::query_as::<_, RefundRow>(
        "SELECT * FROM refunds WHERE idempotency_key = $1",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
