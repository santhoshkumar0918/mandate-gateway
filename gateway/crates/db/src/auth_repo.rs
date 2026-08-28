use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

/// A platform account: merchant, buyer agent, or admin.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AccountRow {
    pub account_id: Uuid,
    pub role: String,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub tenant_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
pub struct AuthRepo {
    pool: PgPool,
}

impl AuthRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Hashes `password` with argon2id (random salt) and inserts a new
    /// account. Returns the created row. Fails with a query error on a
    /// duplicate email (UNIQUE constraint).
    pub async fn create_account(
        &self,
        role: &str,
        email: &str,
        name: &str,
        password: &str,
        tenant_id: Option<&str>,
    ) -> Result<AccountRow, DbError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| DbError::Serialization(format!("password hash failed: {e}")))?
            .to_string();

        let account_id = Uuid::new_v4();
        let row = sqlx::query_as::<_, AccountRow>(
            "INSERT INTO accounts (account_id, role, email, name, password_hash, tenant_id) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING account_id, role, email, name, password_hash, tenant_id, created_at",
        )
        .bind(account_id)
        .bind(role)
        .bind(email)
        .bind(name)
        .bind(&password_hash)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<AccountRow>, DbError> {
        let row = sqlx::query_as::<_, AccountRow>(
            "SELECT account_id, role, email, name, password_hash, tenant_id, created_at \
             FROM accounts WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Verifies `password` against the stored argon2id hash. Returns the
    /// account only if the email exists and the password matches.
    pub async fn verify_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<AccountRow>, DbError> {
        let Some(row) = self.find_by_email(email).await? else {
            return Ok(None);
        };
        let parsed = PasswordHash::new(&row.password_hash)
            .map_err(|e| DbError::Serialization(format!("stored hash invalid: {e}")))?;
        match Argon2::default().verify_password(password.as_bytes(), &parsed) {
            Ok(()) => Ok(Some(row)),
            Err(_) => Ok(None),
        }
    }
}
