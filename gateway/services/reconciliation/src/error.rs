use thiserror::Error;

/// Errors during reconciliation.
#[derive(Debug, Error)]
pub enum ReconciliationError {
    #[error("no intent found for order {order_id}")]
    IntentNotFound { order_id: String },

    #[error("mismatch already resolved: {mismatch_id}")]
    AlreadyResolved { mismatch_id: String },

    #[error("refund API error: {0}")]
    RefundFailed(String),

    #[error("database error: {0}")]
    Db(#[from] db::DbError),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
