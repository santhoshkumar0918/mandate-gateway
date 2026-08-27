use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// A signed, per-purchase authorization produced by the merchant gateway.
///
/// The mandate is a *standing* scope of trust ("this agent may spend up to X
/// in these categories"). A `PurchaseAuth` is the *one-shot* authorization
/// that lets a single, specific purchase actually move money. It binds the
/// purchase's amount, product, and category to a **fresh nonce**, so replaying
/// the same authorization cannot authorize a second, duplicate payment.
///
/// # Money invariants
///
/// - No purchase may execute without a `PurchaseAuth` whose signature verifies
///   against the merchant signing key.
/// - Each `PurchaseAuth` carries a unique `nonce` that must be consumed
///   exactly once; reusing a nonce is a replay attack and must be rejected.
/// - The authorization's `amount` and `category` are the values the policy
///   engine checks against the mandate — the caller never gets to argue a
///   different amount after the fact.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PurchaseAuth {
    /// Unique identifier for this one-shot authorization.
    pub auth_id: Uuid,
    /// The mandate this purchase is drawn against.
    pub mandate_id: Uuid,
    /// Fresh per-purchase nonce — replay protection token.
    pub nonce: String,
    /// Amount being authorized, in currency subunits (paise for INR).
    pub amount: i64,
    /// ISO 4217 currency code.
    pub currency: String,
    /// Product being purchased.
    pub product_id: String,
    /// Category of the product (must be within the mandate's scope).
    pub category: String,
    /// When the authorization was issued.
    pub created_at: DateTime<Utc>,
    /// Ed25519 signature over the canonical JSON of the fields above,
    /// excluding `signature` itself.
    pub signature: Vec<u8>,
}

impl PurchaseAuth {
    /// Creates a new unsigned purchase authorization.
    ///
    /// The gateway must sign it with
    /// [`MandateSigner::sign_auth`](crate::MandateSigner::sign_auth) before it
    /// can authorize a payment.
    pub fn new(
        mandate_id: Uuid,
        amount: i64,
        currency: &str,
        product_id: &str,
        category: &str,
    ) -> Self {
        Self {
            auth_id: Uuid::new_v4(),
            mandate_id,
            nonce: Uuid::new_v4().to_string(),
            amount,
            currency: currency.to_string(),
            product_id: product_id.to_string(),
            category: category.to_string(),
            created_at: Utc::now(),
            signature: Vec::new(),
        }
    }

    /// The fields to be signed, excluding `signature` — the canonical payload.
    ///
    /// Serializing this struct to JSON produces the exact bytes the signature
    /// covers. Field order is fixed by struct declaration, so the same auth
    /// always yields identical bytes.
    pub fn signing_payload(&self) -> Result<Vec<u8>, serde_json::Error> {
        let payload = AuthSigningPayload {
            auth_id: self.auth_id,
            mandate_id: self.mandate_id,
            nonce: &self.nonce,
            amount: self.amount,
            currency: &self.currency,
            product_id: &self.product_id,
            category: &self.category,
            created_at: self.created_at,
        };
        serde_json::to_vec(&payload)
    }
}

/// The subset of `PurchaseAuth` fields that get signed.
///
/// Excludes `signature` — signing the signature would be circular. This struct
/// guarantees the signing payload is always built from the same fields in the
/// same order, regardless of how the full `PurchaseAuth` is serialized.
#[derive(Serialize)]
struct AuthSigningPayload<'a> {
    auth_id: Uuid,
    mandate_id: Uuid,
    nonce: &'a str,
    amount: i64,
    currency: &'a str,
    product_id: &'a str,
    category: &'a str,
    created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_auth_generates_fresh_nonce() {
        let a = PurchaseAuth::new(Uuid::new_v4(), 100, "INR", "prod", "electronics");
        let b = PurchaseAuth::new(Uuid::new_v4(), 100, "INR", "prod", "electronics");
        assert_ne!(a.nonce, b.nonce);
        assert!(a.signature.is_empty());
    }

    #[test]
    fn signing_payload_excludes_signature() {
        let a = PurchaseAuth::new(Uuid::new_v4(), 100, "INR", "prod", "electronics");
        let payload = a.signing_payload().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert!(parsed.get("signature").is_none());
    }
}
