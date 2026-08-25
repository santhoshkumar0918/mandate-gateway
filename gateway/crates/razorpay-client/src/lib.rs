//! # Razorpay Client
//!
//! HTTP client for Razorpay test-mode APIs: Orders, Payments, and Refunds.
//!
//! Uses Basic auth (`key_id:key_secret`) and targets Razorpay's test base URL.
//! All API calls go through [`reqwest`] with structured error handling.
//!
//! # Test mode
//!
//! This client is designed exclusively for Razorpay test mode. It uses test UPI
//! IDs (`success@razorpay`, `failure@razorpay`) and never touches live money.
//!
//! # Idempotency
//!
//! Every order creation accepts an optional `idempotency_key` to prevent
//! duplicate orders from retry logic.

pub mod error;
pub mod orders;
pub mod payments;
pub mod refunds;
pub mod types;

pub use error::RazorpayError;
pub use orders::OrdersApi;
pub use payments::PaymentsApi;
pub use refunds::RefundsApi;
pub use types::{Order, Payment, Refund};
