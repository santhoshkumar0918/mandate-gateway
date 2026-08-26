use thiserror::Error;

/// Errors from database operations.
///
/// Every variant is recoverable — no panics. The gateway server maps these
/// to appropriate HTTP status codes.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("database query failed: {0}")]
    Query(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("mandate not found: {0}")]
    NotFound(String),
}
