use crate::money::Money;
use thiserror::Error;

/// Everything that can go wrong when building or applying a transaction.
#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("insufficient funds: needed {needed}, available {available}")]
    InsufficientFunds { needed: Money, available: Money },

    #[error("arithmetic overflow")]
    Overflow,

    #[error("transaction does not balance (off by {difference} cents)")]
    Unbalanced { difference: i128 },

    #[error("transaction has no postings")]
    Empty,

    #[error("idempotency key was already used with a different request")]
    KeyConflict,
}
