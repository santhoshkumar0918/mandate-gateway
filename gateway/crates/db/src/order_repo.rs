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

/// An order joined with its fulfillment status and the buyer intent that
/// produced it. Fulfillment is derived from the fulfillments table (NULL row
/// => unfulfilled); the intent supplies product + reasoning.
#[derive(Debug, sqlx::FromRow)]
pub struct OrderViewRow {
    pub order_id: String,
    pub mandate_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub fulfilled_at: Option<DateTime<Utc>>,
    pub proof: Option<String>,
    pub product_id: Option<String>,
    pub category: Option<String>,
    pub reasoning: Option<String>,
    pub selection_method: Option<String>,
}

/// Lists recent orders with their fulfillment + intent detail.
pub async fn list_recent(
    pool: &PgPool,
    limit: i32,
) -> Result<Vec<OrderViewRow>, DbError> {
    let rows = sqlx::query_as::<_, OrderViewRow>(
        r#"SELECT o.order_id,
                  o.mandate_id,
                  o.amount,
                  o.currency,
                  o.status,
                  o.created_at,
                  f.fulfilled_at,
                  f.proof,
                  i.product_id,
                  i.category,
                  i.reasoning,
                  i.selection_method
           FROM orders o
           LEFT JOIN fulfillments f ON o.order_id = f.order_id
           LEFT JOIN intents i ON o.order_id = i.order_id
           ORDER BY o.created_at DESC
           LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
