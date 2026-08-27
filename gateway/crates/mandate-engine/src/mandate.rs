use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Frequency of a mandate — one-time or recurring.
///
/// Mirrors the frequency field in Razorpay Reserve Pay's SBMD shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    OneTime,
    Recurring,
}

/// Lifecycle status of a mandate.
///
/// Transitions: Active → (Revoked | Expired | Exhausted).
/// A mandate starts as Active upon issuance and moves to a terminal state
/// when it can no longer authorize payments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MandateStatus {
    Active,
    Revoked,
    Expired,
    Exhausted,
}

/// Parameters for creating a new mandate.
///
/// Groups the caller-supplied fields so [`Mandate::new`] stays clean.
/// Fields like `mandate_id`, `issued_at`, `spent_amount`, `status`, `nonce`,
/// and `signature` are set automatically — the caller never provides them.
#[derive(Debug, Clone)]
pub struct NewMandate {
    pub user_id: String,
    pub merchant_id: String,
    pub buyer_agent_id: String,
    pub max_amount: i64,
    pub currency: String,
    pub scope: Vec<String>,
    pub frequency: Frequency,
    pub expires_at: DateTime<Utc>,
}

/// A signed, scoped permission slip that gates every money-moving action.
///
/// Modeled on Razorpay Reserve Pay's SBMD shape (max_amount, expiry, frequency),
/// extended with scope and agent identity fields. The signature is an Ed25519
/// signature over the canonical JSON of all fields *except* `signature` itself.
///
/// # Invariants
///
/// - `spent_amount` starts at 0 and only increases.
/// - `spent_amount` must never exceed `max_amount`.
/// - Once status leaves `Active`, it cannot return.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Mandate {
    pub mandate_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// The human who authorized this mandate.
    pub user_id: String,
    /// The merchant this mandate is scoped to.
    pub merchant_id: String,
    /// Identity of the requesting buyer agent.
    pub buyer_agent_id: String,
    /// Maximum amount in currency subunits (paise for INR).
    pub max_amount: i64,
    /// ISO 4217 currency code.
    pub currency: String,
    /// Category/SKU whitelist — the agent can only purchase within these scopes.
    pub scope: Vec<String>,
    pub frequency: Frequency,
    /// Running counter of how much has been spent against this mandate.
    pub spent_amount: i64,
    pub status: MandateStatus,
    /// Unique value for replay protection — each nonce can only be used once.
    pub nonce: String,
    /// Ed25519 signature over canonical JSON of all fields above.
    pub signature: Vec<u8>,
}

impl Mandate {
    /// Creates a new unsigned mandate from the given parameters.
    ///
    /// The caller must sign it with [`MandateSigner`](crate::MandateSigner)
    /// before it can authorize any payment. Returns the mandate with
    /// `Active` status and zero spent.
    pub fn new(params: NewMandate) -> Self {
        Self {
            mandate_id: Uuid::new_v4(),
            issued_at: Utc::now(),
            expires_at: params.expires_at,
            user_id: params.user_id,
            merchant_id: params.merchant_id,
            buyer_agent_id: params.buyer_agent_id,
            max_amount: params.max_amount,
            currency: params.currency,
            scope: params.scope,
            frequency: params.frequency,
            spent_amount: 0,
            status: MandateStatus::Active,
            nonce: Uuid::new_v4().to_string(),
            signature: Vec::new(),
        }
    }

    /// Returns the immutable fields to be signed, excluding the signature itself.
    ///
    /// This is the canonical representation — serializing these fields to
    /// deterministic JSON and signing the bytes produces the mandate signature.
    ///
    /// Mutable fields (`spent_amount`, `status`, `nonce`) are deliberately
    /// excluded: the signature must remain valid across the mandate's entire
    /// lifecycle, not just at issuance.
    pub fn signing_payload(&self) -> Result<Vec<u8>, serde_json::Error> {
        let payload = SigningPayload {
            mandate_id: self.mandate_id,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            user_id: &self.user_id,
            merchant_id: &self.merchant_id,
            buyer_agent_id: &self.buyer_agent_id,
            max_amount: self.max_amount,
            currency: &self.currency,
            scope: &self.scope,
            frequency: &self.frequency,
        };
        serde_json::to_vec(&payload)
    }

    /// Checks whether the mandate is currently usable for a payment.
    ///
    /// Does NOT verify the signature — that's [`MandateSigner::verify`](crate::MandateSigner::verify)'s job.
    /// This only checks temporal and lifecycle constraints.
    pub fn is_usable(&self) -> bool {
        self.status == MandateStatus::Active && Utc::now() < self.expires_at
    }
}

/// The subset of mandate fields that get signed.
///
/// Excludes `signature` itself — signing the signature would be circular.
/// Excludes mutable lifecycle fields (`spent_amount`, `status`, `nonce`)
/// — the signature must remain valid across the mandate's entire lifecycle.
#[derive(Serialize)]
struct SigningPayload<'a> {
    mandate_id: Uuid,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    user_id: &'a str,
    merchant_id: &'a str,
    buyer_agent_id: &'a str,
    max_amount: i64,
    currency: &'a str,
    scope: &'a [String],
    frequency: &'a Frequency,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> NewMandate {
        NewMandate {
            user_id: "user-1".into(),
            merchant_id: "merchant-1".into(),
            buyer_agent_id: "agent-1".into(),
            max_amount: 50_000,
            currency: "INR".into(),
            scope: vec!["electronics".into()],
            frequency: Frequency::OneTime,
            expires_at: Utc::now() + chrono::Duration::hours(1),
        }
    }

    #[test]
    fn new_mandate_defaults() {
        let mandate = Mandate::new(test_params());

        assert_eq!(mandate.status, MandateStatus::Active);
        assert_eq!(mandate.spent_amount, 0);
        assert!(mandate.is_usable());
        assert!(mandate.signature.is_empty());
    }

    #[test]
    fn expired_mandate_not_usable() {
        let mut params = test_params();
        params.expires_at = Utc::now() - chrono::Duration::hours(1);
        let mut mandate = Mandate::new(params);
        mandate.status = MandateStatus::Active;

        assert!(!mandate.is_usable());
    }

    #[test]
    fn signing_payload_excludes_signature() {
        let mandate = Mandate::new(test_params());

        let payload = mandate.signing_payload().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert!(parsed.get("signature").is_none());
    }
}
