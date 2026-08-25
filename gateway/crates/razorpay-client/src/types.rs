use serde::{Deserialize, Serialize};

/// Razorpay Order object.
///
/// Created before a payment — represents the merchant's intent to receive
/// a specific amount. Maps directly to Razorpay's Orders API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub entity: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

/// Razorpay Payment object.
///
/// Represents a completed or attempted payment against an order.
/// The `method` field indicates how the payment was made (upi, card, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub entity: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub order_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

/// Razorpay Refund object.
///
/// Represents a refund issued against a payment. Created via the Refunds API
/// and tracked for reconciliation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refund {
    pub id: String,
    pub entity: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub payment_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

/// Shared Razorpay API error response body.
#[derive(Debug, Deserialize)]
pub(crate) struct ApiErrorResponse {
    pub error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApiErrorDetail {
    pub code: String,
    pub description: String,
}
