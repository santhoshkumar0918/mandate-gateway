use reqwest::Client;

use crate::error::RazorpayError;
use crate::types::{ApiErrorResponse, Order, Payment, Refund};

const BASE_URL: &str = "https://api.razorpay.com/v1";

pub struct RazorpayClient {
    client: Client,
    auth_header: String,
}

impl RazorpayClient {
    pub fn new(key_id: &str, key_secret: &str) -> Self {
        let credentials = format!("{}:{}", key_id, key_secret);
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, credentials);
        Self {
            client: Client::new(),
            auth_header: format!("Basic {}", encoded),
        }
    }

    pub async fn create_order(
        &self,
        amount: i64,
        currency: &str,
        receipt: Option<&str>,
    ) -> Result<Order, RazorpayError> {
        let mut body = serde_json::json!({
            "amount": amount,
            "currency": currency,
            "payment_capture": "0",
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
        let client = RazorpayClient::new("test_id", "test_secret");
        assert!(client.auth_header.starts_with("Basic "));
    }

    #[test]
    fn auth_header_is_base64() {
        let client = RazorpayClient::new("test_id", "test_secret");
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "test_id:test_secret",
        );
        assert_eq!(client.auth_header, format!("Basic {}", encoded));
    }
}
