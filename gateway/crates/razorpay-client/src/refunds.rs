use reqwest::Client;

use crate::error::RazorpayError;
use crate::types::{ApiErrorResponse, Refund};

const BASE_URL: &str = "https://api.razorpay.com/v1";

/// Razorpay Refunds API client.
///
/// Handles refund creation and retrieval. Used by the reconciliation flow
/// to issue refunds when intent-vs-outcome mismatches are detected.
pub struct RefundsApi {
    client: Client,
    auth_header: String,
}

impl RefundsApi {
    /// Creates a new Refunds API client.
    pub fn new(key_id: &str, key_secret: &str) -> Self {
        let credentials = format!("{}:{}", key_id, key_secret);
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, credentials);
        Self {
            client: Client::new(),
            auth_header: format!("Basic {}", encoded),
        }
    }

    /// Issues a refund for a payment.
    ///
    /// The `payment_id` is the Razorpay payment to refund. `amount` is
    /// optional — if None, a full refund is issued.
    pub async fn create_refund(
        &self,
        payment_id: &str,
        amount: Option<i64>,
    ) -> Result<Refund, RazorpayError> {
        let mut body = serde_json::json!({});
        if let Some(a) = amount {
            body["amount"] = serde_json::json!(a);
        }

        let resp = self
            .client
            .post(format!("{}/payments/{}/refund", BASE_URL, payment_id))
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(resp.json().await?)
        } else {
            let err: ApiErrorResponse = resp.json().await?;
            Err(RazorpayError::Api {
                code: err.error.code,
                description: err.error.description,
            })
        }
    }

    /// Fetches a refund by ID.
    pub async fn fetch_refund(
        &self,
        payment_id: &str,
        refund_id: &str,
    ) -> Result<Refund, RazorpayError> {
        let resp = self
            .client
            .get(format!(
                "{}/payments/{}/refunds/{}",
                BASE_URL, payment_id, refund_id
            ))
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(resp.json().await?)
        } else {
            let err: ApiErrorResponse = resp.json().await?;
            Err(RazorpayError::Api {
                code: err.error.code,
                description: err.error.description,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_header_format() {
        let api = RefundsApi::new("test_id", "test_secret");
        assert!(api.auth_header.starts_with("Basic "));
    }
}
