pub mod audit_repo;
pub mod error;
pub mod intent_repo;
pub mod mandate_repo;
pub mod mismatch_repo;
pub mod order_repo;
pub mod payment_repo;
pub mod refund_repo;

pub use error::DbError;
pub use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct PgNonceChecker {
    pool: PgPool,
}

impl PgNonceChecker {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl policy_engine::NonceChecker for PgNonceChecker {
    fn is_nonce_fresh(&self, nonce: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.pool.clone();
        let nonce = nonce.to_string();
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                mandate_repo::nonce_exists(&pool, &nonce).await
            })
        })?;
        Ok(!result)
    }
}
