use std::fmt;

pub trait NonceChecker: fmt::Debug + Send + Sync {
    fn is_nonce_fresh(&self, nonce: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
}
