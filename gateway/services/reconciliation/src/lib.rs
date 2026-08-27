//! # Reconciliation
//!
//! Detects intent-vs-outcome mismatches after a purchase completes.
//!
//! The interesting failure mode: the agent stays *inside* the mandate but the
//! outcome is still wrong (price drifted, wrong variant, inventory raced out).
//! This module catches that and triggers recovery (refund).
//!
//! ## Concurrency handling
//!
//! Uses row-level locking (Postgres `SELECT ... FOR UPDATE`) to serialize
//! concurrent mismatch resolution per mandate. No separate queue system needed.

pub mod error;
pub mod matcher;
pub mod refund_provider;
pub mod service;
pub mod types;

pub use error::ReconciliationError;
pub use matcher::MismatchDetector;
pub use refund_provider::{IssuedRefund, RazorpayRefundProvider, RefundProvider};
pub use service::ReconcileService;
pub use types::{Intent, Mismatch, MismatchKind, MismatchStatus, Outcome};
