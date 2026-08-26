//! # Policy Engine
//!
//! Deterministic allow/block/escalate logic that sits between a purchase
//! request and an actual payment call. No LLM, no ML — plain code that
//! checks a mandate's limits and decides whether to proceed.
//!
//! ## Design invariant
//!
//! This crate has **zero AI/ML dependencies**. The absence of an LLM
//! in `Cargo.toml` *is* the design decision. Every decision is auditable
//! line by line.
//!
//! ## Decision model
//!
//! Every purchase request produces exactly one [`Decision`]:
//! - **Allow** — mandate is valid, request is within bounds, proceed.
//! - **Block** — mandate violation detected, hard stop.
//! - **Escalate** — ambiguous case, needs human review.

pub mod decision;
pub mod evaluator;
pub mod nonce_checker;
pub mod rules;

pub use decision::{Decision, DecisionKind};
pub use evaluator::PolicyEvaluator;
pub use nonce_checker::NonceChecker;
pub use rules::RuleViolation;
