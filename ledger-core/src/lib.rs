//! Core domain logic for a small double-entry ledger.
//!
//! - [`money`]: amounts as whole cents.
//! - [`transaction`]: balanced sets of postings.
//! - [`ledger`]: account balances, updated atomically.
//! - [`idempotent`]: safe retries using idempotency keys.
//! - [`error`]: the single error type.
//!
//! This crate has no I/O and no web dependencies.

pub mod error;
pub mod idempotent;
pub mod ledger;
pub mod money;
pub mod transaction;
