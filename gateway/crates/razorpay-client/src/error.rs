use thiserror::Error;

/// Errors from Razorpay API calls.
///
/// Every variant is recoverable — no panics on API failures. The caller
/// decides whether to retry, escalate, or log.
#[derive(Debug, Error)]
pub enum RazorpayError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Razorpay API error {code}: {description}")]
    Api { code: String, description: String },

    #[error("authentication failed — check RAZORPAY_KEY_ID and RAZORPAY_KEY_SECRET")]
    AuthFailed,

    #[error("request timeout")]
    Timeout,

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
