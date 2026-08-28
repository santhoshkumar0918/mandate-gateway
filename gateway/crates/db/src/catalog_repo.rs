use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::error::DbError;

/// A catalog product owned by a merchant. Mirrors the ACP-shaped `Product`
/// served by the gateway; the gateway maps this to its own type.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CatalogProductRow {
    pub product_id: String,
    pub merchant_id: String,
    pub offer_id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub price: i64,
    pub currency: String,
    pub availability: String,
    pub inventory_count: i64,
    pub seller_name: String,
    pub updated_at: DateTime<Utc>,
}

/// The catalog is persisted per merchant — every read is scoped to a tenant.
#[derive(Clone)]
pub struct CatalogRepo {
    pool: PgPool,
}

impl CatalogRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, merchant_id: &str) -> Result<Vec<CatalogProductRow>, DbError> {
        let rows = sqlx::query_as::<_, CatalogProductRow>(
            "SELECT product_id, merchant_id, offer_id, title, description, category, \
             price, currency, availability, inventory_count, seller_name, updated_at \
             FROM catalog WHERE merchant_id = $1 ORDER BY product_id",
        )
        .bind(merchant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get(
        &self,
        merchant_id: &str,
        product_id: &str,
    ) -> Result<Option<CatalogProductRow>, DbError> {
        let row = sqlx::query_as::<_, CatalogProductRow>(
            "SELECT product_id, merchant_id, offer_id, title, description, category, \
             price, currency, availability, inventory_count, seller_name, updated_at \
             FROM catalog WHERE merchant_id = $1 AND product_id = $2",
        )
        .bind(merchant_id)
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Bumps a product's price and returns the refreshed row. Used by the
    /// catalog-drift failure scenario. The write is a plain UPDATE — the
    /// drift is intentional and still audit-logged upstream.
    pub async fn set_price(
        &self,
        merchant_id: &str,
        product_id: &str,
        new_price: i64,
    ) -> Result<Option<CatalogProductRow>, DbError> {
        let row = sqlx::query_as::<_, CatalogProductRow>(
            "UPDATE catalog SET price = $3, updated_at = NOW() \
             WHERE merchant_id = $1 AND product_id = $2 \
             RETURNING product_id, merchant_id, offer_id, title, description, category, \
             price, currency, availability, inventory_count, seller_name, updated_at",
        )
        .bind(merchant_id)
        .bind(product_id)
        .bind(new_price)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }
}
