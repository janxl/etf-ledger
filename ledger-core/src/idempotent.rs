use std::collections::HashMap;

use crate::error::LedgerError;
use crate::ledger::Ledger;
use crate::transaction::Transaction;

/// What happened to a submitted request.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The transaction was applied to the ledger.
    Applied,
    /// The key was seen before with the same transaction; nothing changed.
    Replayed,
}

/// A [`Ledger`] that applies each idempotency key at most once.
///
/// Only successful requests are remembered, so a request that failed can be
/// retried with the same key. Keys are kept in memory and never expire.
#[derive(Debug, Default)]
pub struct IdempotentLedger {
    ledger: Ledger,
    seen: HashMap<String, Transaction>,
}

impl IdempotentLedger {
    /// An empty ledger that has seen no keys.
    pub fn new() -> IdempotentLedger {
        IdempotentLedger::default()
    }

    /// A read-only view of the underlying ledger.
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// Applies `tx` the first time `key` is seen, and replays it afterwards.
    ///
    /// # Errors
    ///
    /// - [`LedgerError::KeyConflict`] if `key` was used before with a different
    ///   transaction.
    /// - Any error from [`Ledger::apply`]. The key is not recorded in that case.
    ///
    /// # Examples
    ///
    /// ```
    /// use ledger_core::idempotent::{IdempotentLedger, Outcome};
    /// use ledger_core::money::Money;
    /// use ledger_core::transaction::{Posting, Transaction};
    ///
    /// let buy = || {
    ///     Transaction::new(vec![
    ///         Posting { account: "cash".into(), amount: Money(-1000) },
    ///         Posting { account: "etf:VWRL".into(), amount: Money(1000) },
    ///     ])
    ///     .unwrap()
    /// };
    ///
    /// let mut book = IdempotentLedger::new();
    /// assert_eq!(book.submit("k1", buy()).unwrap(), Outcome::Applied);
    /// assert_eq!(book.submit("k1", buy()).unwrap(), Outcome::Replayed);
    /// assert_eq!(book.ledger().balance("etf:VWRL").0, 1000); // applied once
    /// ```
    pub fn submit(&mut self, key: &str, tx: Transaction) -> Result<Outcome, LedgerError> {
        match self.seen.get(key) {
            Some(previous) if *previous == tx => Ok(Outcome::Replayed),
            Some(_) => Err(LedgerError::KeyConflict),
            None => {
                self.ledger.apply(&tx)?;
                self.seen.insert(key.to_string(), tx);
                Ok(Outcome::Applied)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Money;
    use crate::transaction::Posting;

    fn tx(postings: &[(&str, i64)]) -> Transaction {
        let postings = postings
            .iter()
            .map(|(account, cents)| Posting {
                account: account.to_string(),
                amount: Money(*cents),
            })
            .collect();
        Transaction::new(postings).unwrap()
    }

    #[test]
    fn same_key_applies_only_once() {
        let mut book = IdempotentLedger::new();
        let buy = tx(&[("cash", -1000), ("etf:VWRL", 1000)]);

        assert_eq!(book.submit("key-1", buy.clone()).unwrap(), Outcome::Applied);
        assert_eq!(book.submit("key-1", buy).unwrap(), Outcome::Replayed);

        assert_eq!(book.ledger().balance("etf:VWRL").0, 1000); // not 2000
    }

    #[test]
    fn same_key_with_different_payload_is_a_conflict() {
        let mut book = IdempotentLedger::new();
        book.submit("key-1", tx(&[("cash", -1000), ("etf:VWRL", 1000)]))
            .unwrap();

        let result = book.submit("key-1", tx(&[("cash", -9999), ("etf:VWRL", 9999)]));

        assert!(matches!(result, Err(LedgerError::KeyConflict)));
        assert_eq!(book.ledger().balance("etf:VWRL").0, 1000);
    }

    #[test]
    fn different_keys_both_apply() {
        let mut book = IdempotentLedger::new();
        book.submit("key-1", tx(&[("cash", -1000), ("etf:VWRL", 1000)]))
            .unwrap();
        book.submit("key-2", tx(&[("cash", -1000), ("etf:VWRL", 1000)]))
            .unwrap();

        assert_eq!(book.ledger().balance("etf:VWRL").0, 2000);
    }
}
