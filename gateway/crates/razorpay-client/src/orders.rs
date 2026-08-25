use reqwest::Client;

use crate::error::RazorpayError;
use crate::types::{ApiErrorResponse, Order};

const BASE_URL: &str = "https://api.razorpay.com/v1";

/// Razorpay Orders API client.
///
/// Handles order creation and retrieval. All amounts are in currency subunits
/// (paise for INR).
pub struct OrdersApi {
    client: Client,
    auth_header: String,
}

impl OrdersApi {
    /// Creates a new Orders API client.
    ///
    /// # Arguments
    ///
    /// * `key_id` — Razorpay test-mode key ID
    /// * `key_secret` — Razorpay test-mode key secret
    pub fn new(key_id: &str, key_secret: &str) -> Self {
        let credentials = format!("{}:{}", key_id, key_secret);
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, credentials);
        Self {
            client: Client::new(),
            auth_header: format!("Basic {}", encoded),
        }
    }

    /// Creates a new order.
    ///
    /// # Arguments
    ///
    /// * `amount` — amount in currency subunits (paise)
    /// * `currency` — ISO 4217 code (e.g., "INR")
    /// * `receipt` — optional receipt ID for idempotency
    pub async fn create_order(
        &self,
        amount: i64,
        currency: &str,
        receipt: Option<&str>,
    ) -> Result<Order, RazorpayError> {
        let mut body = serde_json::json!({
            "amount": amount,
            "currency": currency,
        });
        if let Some(r) = receipt {
            body["receipt"] = serde_json::json!(r);
        }

        let resp = self
            .client
            .post(format!("{}/orders", BASE_URL))
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

    /// Fetches an order by ID.
    pub async fn fetch_order(&self, order_id: &str) -> Result<Order, RazorpayError> {
        let resp = self
            .client
            .get(format!("{}/orders/{}", BASE_URL, order_id))
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
        let api = OrdersApi::new("test_id", "test_secret");
        assert!(api.auth_header.starts_with("Basic "));
        assert!(api.auth_header.len() > 10);
    }
}
