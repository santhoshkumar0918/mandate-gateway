//! # Database Layer
//!
//! Postgres persistence for the Merchant Agent Gateway. All SQL lives here —
//! no other crate touches the database directly. This is the "adapter" in
//! ports & adapters: the core logic (mandate signing, policy evaluation)
//! doesn't depend on Postgres, but this crate bridges the two worlds.
//!
//! ## Design
//!
//! Each domain entity gets its own repo module with async methods that take
//! a `PgPool` reference. Repos are stateless — the pool handles connection
//! management. All queries use sqlx's compile-time checked macros where
//! possible, falling back to dynamic queries for JSONB fields.

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

/// Postgres-backed nonce checker for replay detection.
///
/// Implements the [`policy_engine::NonceChecker`] trait so the policy evaluator
/// can check nonce freshness without depending on Postgres directly.
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
        // We need to block on the async DB call from a sync trait method.
        // Use tokio's Handle::block_on for this — acceptable because the
        // evaluator is always called from a tokio context.
        let pool = self.pool.clone();
        let nonce = nonce.to_string();
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                mandate_repo::nonce_exists(&pool, &nonce).await
            })
        })?;
        Ok(!result) // fresh = nonce does NOT exist
    }
}
