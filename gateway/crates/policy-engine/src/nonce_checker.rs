use std::fmt;

/// Checks whether a mandate's nonce has been seen before.
///
/// This is the nonce replay protection primitive. The policy evaluator calls
/// this before allowing any payment. If the nonce has been used, the request
/// is blocked — no payment proceeds.
///
/// # Why a trait?
///
/// The policy engine doesn't own the database. This trait lets the evaluator
/// ask "is this nonce fresh?" without knowing how the answer is stored.
/// The gateway provides a Postgres-backed implementation; tests provide
/// in-memory fakes.
pub trait NonceChecker: fmt::Debug + Send + Sync {
    /// Returns `Ok(true)` if the nonce is fresh (never seen before).
    /// Returns `Ok(false)` if the nonce has been used (replay detected).
    /// Returns `Err` if the check itself failed (DB error, etc).
    fn is_nonce_fresh(&self, nonce: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
}
