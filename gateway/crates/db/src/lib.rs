//! # Database Layer
//!
//! Postgres persistence for the Merchant Agent Gateway. All SQL lives here —
//! no other crate touches the database directly. This is the "adapter" in
//! ports & adapters: the core logic (mandate signing, policy evaluation)
//! doesn't depend on Postgres, but this crate bridges the two worlds.
//!
//! ## Design
//!
//! Each domain entity gets its own repo module with async methods that take
//! a `PgPool` reference. Repos are stateless — the pool handles connection
//! management. All queries use sqlx's compile-time checked macros where
//! possible, falling back to dynamic queries for JSONB fields.

pub mod audit_repo;
pub mod error;
pub mod intent_repo;
pub mod mandate_repo;
pub mod mismatch_repo;
pub mod order_repo;
pub mod payment_repo;
pub mod refund_repo;

pub use error::DbError;
pub use sqlx::PgPool;
