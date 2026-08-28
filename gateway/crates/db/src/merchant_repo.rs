use sqlx::PgPool;

use crate::error::DbError;

/// A merchant tenant. The manifest endpoint reads from this table so a
/// discovering agent sees a persisted identity, not a hardcoded constant.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MerchantRow {
    pub merchant_id: String,
    pub name: String,
    pub supported_scopes: Vec<String>,
    pub capability_version: String,
    pub catalog_endpoint: String,
    pub mandate_endpoint: String,
}

#[derive(Clone)]
pub struct MerchantRepo {
    pool: PgPool,
}

impl MerchantRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, merchant_id: &str) -> Result<Option<MerchantRow>, DbError> {
        let row = sqlx::query_as::<_, MerchantRow>(
            "SELECT merchant_id, name, supported_scopes, capability_version, \
             catalog_endpoint, mandate_endpoint FROM merchants WHERE merchant_id = $1",
        )
        .bind(merchant_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }
}
