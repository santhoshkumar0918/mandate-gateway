//! # Mandate Engine
//!
//! Trust-critical core of the Merchant Agent Gateway.
//!
//! Handles Ed25519 signing and verification of mandates — the cryptographically
//! scoped permission slips that gate every money-moving action. No payment
//! executes without a verified, unexpired, in-scope mandate.
//!
//! ## Invariants
//!
//! - A mandate is only valid if its signature verifies against the signing key.
//! - A mandate cannot be tampered with after issuance — canonical JSON ensures
//!   reproducible byte output independent of field ordering.
//! - Replay protection via nonces: each nonce can only be used once per mandate.
//! - Expired or exhausted mandates are rejected at verification time, not by
//!   convention.

pub mod error;
pub mod mandate;
pub mod purchase_auth;
pub mod signing;

pub use error::MandateError;
pub use mandate::{Frequency, Mandate, MandateStatus, NewMandate};
pub use purchase_auth::PurchaseAuth;
pub use signing::MandateSigner;
