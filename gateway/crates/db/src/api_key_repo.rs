use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

/// A scoped API key owned by an account. The raw secret is never persisted —
/// only its SHA-256 hash (see `ApiKeyRepo::create_key`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiKeyRow {
    pub key_id: Uuid,
    pub account_id: Uuid,
    pub label: String,
    pub scopes: serde_json::Value,
    pub tenant_id: Option<String>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct ApiKeyRepo {
    pool: PgPool,
}

impl ApiKeyRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Generates a new raw key (`magw_<64 hex>`), stores only its hash, and
    /// returns the raw key (shown to the caller exactly once) plus the row.
    pub async fn create_key(
        &self,
        account_id: Uuid,
        label: &str,
        scopes: Vec<String>,
        tenant_id: Option<&str>,
    ) -> Result<(String, ApiKeyRow), DbError> {
        let mut bytes = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut bytes);
        let raw = format!("magw_{}", hex::encode(bytes));

        let hash = Sha256::digest(raw.as_bytes());
        let key_hash = hex::encode(hash);

        let key_id = Uuid::new_v4();
        let row = sqlx::query_as::<_, ApiKeyRow>(
            "INSERT INTO api_keys (key_id, account_id, key_hash, label, scopes, tenant_id) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING key_id, account_id, label, scopes, tenant_id, revoked, created_at, last_used_at",
        )
        .bind(key_id)
        .bind(account_id)
        .bind(&key_hash)
        .bind(label)
        .bind(serde_json::json!(scopes))
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok((raw, row))
    }

    /// Resolves a presented raw key to its (non-revoked) row, updating
    /// `last_used_at`. Returns None if unknown or revoked.
    pub async fn verify_key(&self, raw: &str) -> Result<Option<ApiKeyRow>, DbError> {
        let hash = hex::encode(Sha256::digest(raw.as_bytes()));
        let row = sqlx::query_as::<_, ApiKeyRow>(
            "UPDATE api_keys SET last_used_at = NOW() \
             WHERE key_id = (SELECT key_id FROM api_keys WHERE key_hash = $1 AND revoked = FALSE) \
             RETURNING key_id, account_id, label, scopes, tenant_id, revoked, created_at, last_used_at",
        )
        .bind(&hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn revoke(&self, key_id: Uuid, account_id: Uuid) -> Result<bool, DbError> {
        let result = sqlx::query("UPDATE api_keys SET revoked = TRUE WHERE key_id = $1 AND account_id = $2")
            .bind(key_id)
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_for_account(&self, account_id: &Uuid) -> Result<Vec<ApiKeyRow>, DbError> {
        let rows = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT key_id, account_id, label, scopes, tenant_id, revoked, created_at, last_used_at \
             FROM api_keys WHERE account_id = $1 ORDER BY created_at DESC",
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
