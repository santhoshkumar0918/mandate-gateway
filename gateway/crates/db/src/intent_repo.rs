use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

/// All positional columns of the intents row; clippy flags argument count on
/// SQL insert helpers like these, so it's allowed here by design.
#[allow(clippy::too_many_arguments)]
pub async fn insert(
    pool: &PgPool,
    intent_id: Uuid,
    mandate_id: Uuid,
    product_id: &str,
    category: &str,
    expected_price: i64,
    currency: &str,
    reasoning: Option<&str>,
    selection_method: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        r#"INSERT INTO intents
            (intent_id, mandate_id, product_id, category, expected_price, currency, reasoning, selection_method)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
    )
    .bind(intent_id)
    .bind(mandate_id)
    .bind(product_id)
    .bind(category)
    .bind(expected_price)
    .bind(currency)
    .bind(reasoning)
    .bind(selection_method)
    .execute(pool)
    .await?;

    Ok(())
}

/// Links an intent to the order it eventually produced (the order_id is not
/// known at intent-insert time because it comes back from Razorpay after
/// intent creation).
pub async fn link_to_order(
    pool: &PgPool,
    intent_id: Uuid,
    order_id: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE intents SET order_id = $1 WHERE intent_id = $2")
        .bind(order_id)
        .bind(intent_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct IntentRow {
    pub intent_id: Uuid,
    pub mandate_id: Uuid,
    pub product_id: String,
    pub category: String,
    pub expected_price: i64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub reasoning: Option<String>,
    pub selection_method: Option<String>,
    pub order_id: Option<String>,
}

pub async fn find_by_id(
    pool: &PgPool,
    intent_id: Uuid,
) -> Result<Option<IntentRow>, DbError> {
    let row = sqlx::query_as::<_, IntentRow>(
        "SELECT * FROM intents WHERE intent_id = $1",
    )
    .bind(intent_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
