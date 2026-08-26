use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

pub async fn insert(
    pool: &PgPool,
    intent_id: Uuid,
    mandate_id: Uuid,
    product_id: &str,
    category: &str,
    expected_price: i64,
    currency: &str,
) -> Result<(), DbError> {
    sqlx::query(
        r#"INSERT INTO intents
            (intent_id, mandate_id, product_id, category, expected_price, currency)
        VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(intent_id)
    .bind(mandate_id)
    .bind(product_id)
    .bind(category)
    .bind(expected_price)
    .bind(currency)
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
