use reqwest::Client;

use crate::error::RazorpayError;
use crate::types::{ApiErrorResponse, Payment};

const BASE_URL: &str = "https://api.razorpay.com/v1";

/// Razorpay Payments API client.
///
/// Handles payment capture and retrieval. Uses Razorpay's test UPI IDs
/// (`success@razorpay`, `failure@razorpay`) for test mode.
pub struct PaymentsApi {
    client: Client,
    auth_header: String,
}

impl PaymentsApi {
    /// Creates a new Payments API client.
    pub fn new(key_id: &str, key_secret: &str) -> Self {
        let credentials = format!("{}:{}", key_id, key_secret);
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, credentials);
        Self {
            client: Client::new(),
            auth_header: format!("Basic {}", encoded),
        }
    }

    /// Captures a payment against an order.
    ///
    /// In test mode, use `success@razorpay` for a successful payment or
    /// `failure@razorpay` to simulate a failure.
    pub async fn capture_payment(
        &self,
        order_id: &str,
        amount: i64,
        currency: &str,
        method: &str,
    ) -> Result<Payment, RazorpayError> {
        let body = serde_json::json!({
            "amount": amount,
            "currency": currency,
            "method": method,
            "order_id": order_id,
        });

        let resp = self
            .client
            .post(format!("{}/payments", BASE_URL))
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

    /// Fetches a payment by ID.
    pub async fn fetch_payment(&self, payment_id: &str) -> Result<Payment, RazorpayError> {
        let resp = self
            .client
            .get(format!("{}/payments/{}", BASE_URL, payment_id))
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
        let api = PaymentsApi::new("test_id", "test_secret");
        assert!(api.auth_header.starts_with("Basic "));
    }
}
