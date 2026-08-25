use serde::{Deserialize, Serialize};

/// Merchant manifest — tells a discovering agent what this merchant offers
/// and where to find things.
///
/// Served at `GET /manifest`. No authentication required — this is how
/// agents discover the merchant in the first place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantManifest {
    pub merchant_id: String,
    pub merchant_name: String,
    /// Capability version — signals what the gateway supports (e.g., "1.0").
    pub capability_version: String,
    /// Categories this merchant allows agent purchases against.
    pub supported_scopes: Vec<String>,
    /// Where to fetch the product feed.
    pub catalog_endpoint: String,
    /// Where to request a mandate.
    pub mandate_endpoint: String,
}

/// Product in the merchant's catalog.
///
/// Grounded in the ACP Product Feed Spec (OpenAI + Stripe, Apache 2.0),
/// trimmed to what a lean build needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub product_id: String,
    /// SKU + seller + price, per ACP spec.
    pub offer_id: String,
    /// Max 150 chars, no all-caps (ACP rule).
    pub title: String,
    /// Max 5000 chars (ACP rule).
    pub description: String,
    /// Maps to mandate scope[] whitelist.
    pub category: String,
    /// Currency subunits (paise for INR).
    pub price: i64,
    /// ISO 4217 code.
    pub currency: String,
    pub availability: Availability,
    pub inventory_count: i64,
    pub seller_name: String,
    pub seller_id: String,
    /// Freshness timestamp — the field that deliberately goes stale
    /// in the catalog drift failure scenario.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    InStock,
    OutOfStock,
    Preorder,
}
