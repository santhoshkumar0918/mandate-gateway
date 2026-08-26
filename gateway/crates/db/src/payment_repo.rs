use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

/// Parameters for inserting a payment record.
#[derive(Debug)]
pub struct InsertPaymentParams<'a> {
    pub payment_id: &'a str,
    pub order_id: &'a str,
    pub mandate_id: Uuid,
    pub amount: i64,
    pub currency: &'a str,
    pub status: &'a str,
    pub method: Option<&'a str>,
    pub captured: bool,
}

pub async fn insert(pool: &PgPool, params: &InsertPaymentParams<'_>) -> Result<(), DbError> {
    sqlx::query(
        r#"INSERT INTO payments
            (payment_id, order_id, mandate_id, amount, currency, status, method, captured)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
    )
    .bind(params.payment_id)
    .bind(params.order_id)
    .bind(params.mandate_id)
    .bind(params.amount)
    .bind(params.currency)
    .bind(params.status)
    .bind(params.method)
    .bind(params.captured)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct PaymentRow {
    pub payment_id: String,
    pub order_id: String,
    pub mandate_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub method: Option<String>,
    pub captured: Option<bool>,
    pub created_at: DateTime<Utc>,
}

pub async fn find_by_id(
    pool: &PgPool,
    payment_id: &str,
) -> Result<Option<PaymentRow>, DbError> {
    let row = sqlx::query_as::<_, PaymentRow>(
        "SELECT * FROM payments WHERE payment_id = $1",
    )
    .bind(payment_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
