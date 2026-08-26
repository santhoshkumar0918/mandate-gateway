use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

pub async fn insert(
    pool: &PgPool,
    order_id: &str,
    mandate_id: Uuid,
    amount: i64,
    currency: &str,
    status: &str,
    receipt: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        r#"INSERT INTO orders (order_id, mandate_id, amount, currency, status, receipt)
        VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(order_id)
    .bind(mandate_id)
    .bind(amount)
    .bind(currency)
    .bind(status)
    .bind(receipt)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrderRow {
    pub order_id: String,
    pub mandate_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub receipt: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn find_by_id(
    pool: &PgPool,
    order_id: &str,
) -> Result<Option<OrderRow>, DbError> {
    let row = sqlx::query_as::<_, OrderRow>(
        "SELECT * FROM orders WHERE order_id = $1",
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
