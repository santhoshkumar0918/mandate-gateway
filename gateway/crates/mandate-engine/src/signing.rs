use ed25519_dalek::{Signer, SigningKey, VerifyingKey};

use crate::error::MandateError;
use crate::mandate::Mandate;

/// Ed25519 mandate signer and verifier.
///
/// The gateway holds one [`MandateSigner`] instance backed by a keypair.
/// Every mandate is signed on issuance and verified on every subsequent
/// check — this is the trust-critical primitive the rest of the system
/// depends on.
///
/// # Security
///
/// - The signing key must never leave this module's boundary.
/// - Verification is stateless and can be called from any context.
/// - Signature bytes are compared in constant time via `ed25519-dalek`.
pub struct MandateSigner {
    signing_key: SigningKey,
}

impl MandateSigner {
    /// Creates a new signer from an existing signing key.
    pub fn new(signing_key: SigningKey) -> Self {
        Self { signing_key }
    }

    /// Creates a new signer with a freshly generated keypair.
    ///
    /// The returned [`SigningKey`] must be persisted securely — losing it
    /// means no new mandates can be issued, and existing mandates can't
    /// be re-verified if verification depends on this key.
    pub fn generate() -> (Self, SigningKey) {
        let mut csprng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut csprng);
        let signer = Self::new(signing_key.clone());
        (signer, signing_key)
    }

    /// Returns the verifying (public) key for this signer.
    ///
    /// Distribute this to verifiers — they don't need the signing key.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Signs a mandate's payload and writes the signature into the mandate.
    ///
    /// After signing, the mandate's `signature` field contains the Ed25519
    /// signature over the canonical JSON of all other fields. The mandate
    /// is not valid for payment authorization until this is called.
    ///
    /// # Errors
    ///
    /// Returns [`MandateError::Serialization`] if the signing payload can't
    /// be serialized — this is a programming error, not a runtime condition.
    pub fn sign(&self, mandate: &mut Mandate) -> Result<(), MandateError> {
        let payload = mandate.signing_payload()?;
        let signature = self.signing_key.sign(&payload);
        mandate.signature = signature.to_bytes().to_vec();
        Ok(())
    }

    /// Verifies a mandate's signature against this signer's public key.
    ///
    /// Returns `Ok(true)` if the signature is valid and the mandate hasn't
    /// expired or been revoked. Returns `Ok(false)` if usable but signature
    /// is invalid. Returns `Err` for terminal states (expired, revoked).
    ///
    /// # Invariant
    ///
    /// This must be called before any payment action. A mandate that hasn't
    /// been verified through this method must never be trusted.
    pub fn verify(&self, mandate: &Mandate) -> Result<bool, MandateError> {
        // Check lifecycle first — no point verifying a dead mandate
        if !mandate.is_usable() {
            return Err(MandateError::Expired {
                expires_at: mandate.expires_at.to_rfc3339(),
            });
        }

        // Rebuild the signing payload and verify
        let payload = mandate.signing_payload()?;
        let signature_bytes: [u8; 64] = mandate
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| MandateError::Crypto("invalid signature length".into()))?;

        let signature = ed25519_dalek::Signature::from_bytes(&signature_bytes);

        match self.signing_key.verify(&payload, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mandate::{Frequency, Mandate, NewMandate};
    use chrono::Utc;

    fn test_mandate() -> Mandate {
        Mandate::new(NewMandate {
            user_id: "user-1".into(),
            merchant_id: "merchant-1".into(),
            buyer_agent_id: "agent-1".into(),
            max_amount: 50_000,
            currency: "INR".into(),
            scope: vec!["electronics".into()],
            frequency: Frequency::OneTime,
            expires_at: Utc::now() + chrono::Duration::hours(1),
        })
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let (signer, _) = MandateSigner::generate();
        let mut mandate = test_mandate();

        signer.sign(&mut mandate).unwrap();
        assert!(!mandate.signature.is_empty());

        let result = signer.verify(&mandate).unwrap();
        assert!(result);
    }

    #[test]
    fn tampered_mandate_fails_verification() {
        let (signer, _) = MandateSigner::generate();
        let mut mandate = test_mandate();
        signer.sign(&mut mandate).unwrap();

        // Tamper with max_amount after signing
        mandate.max_amount = 999_999;

        let result = signer.verify(&mandate).unwrap();
        assert!(!result);
    }

    #[test]
    fn wrong_key_fails_verification() {
        let (signer_a, _) = MandateSigner::generate();
        let (signer_b, _) = MandateSigner::generate();
        let mut mandate = test_mandate();

        signer_a.sign(&mut mandate).unwrap();
        let result = signer_b.verify(&mandate).unwrap();
        assert!(!result);
    }

    #[test]
    fn expired_mandate_returns_error() {
        let (signer, _) = MandateSigner::generate();
        let mut mandate = test_mandate();
        mandate.expires_at = Utc::now() - chrono::Duration::hours(1);
        signer.sign(&mut mandate).unwrap();

        let result = signer.verify(&mandate);
        assert!(result.is_err());
    }
}
