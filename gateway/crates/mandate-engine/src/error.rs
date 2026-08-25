use thiserror::Error;

/// Errors that can occur during mandate operations.
///
/// Every variant maps to a specific failure mode in the mandate lifecycle.
/// No variant causes a panic — all are recoverable and auditable.
#[derive(Debug, Error)]
pub enum MandateError {
    #[error("signature verification failed: mandate tampered or wrong key")]
    SignatureInvalid,

    #[error("mandate has expired (expires_at: {expires_at})")]
    Expired { expires_at: String },

    #[error("mandate has been revoked")]
    Revoked,

    #[error("mandate budget exhausted (spent: {spent}, max: {max})")]
    Exhausted { spent: i64, max: i64 },

    #[error("nonce already used — possible replay attack")]
    NonceReplay,

    #[error("mandate scope mismatch: requested category '{category}' not in scope")]
    ScopeMismatch { category: String },

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("crypto error: {0}")]
    Crypto(String),
}
