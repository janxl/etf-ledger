use crate::error::LedgerError;
use crate::money::Money;

/// One line of a transaction: an amount moved into (positive) or out of
/// (negative) a named account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posting {
    pub account: String,
    pub amount: Money, // positive = debit, negative = credit
}

/// A set of postings whose amounts sum to zero.
///
/// The field is private and [`Transaction::new`] checks the rule, so every
/// `Transaction` value is guaranteed to balance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    postings: Vec<Posting>, // private on purpose
}

impl Transaction {
    /// Builds a transaction, or explains why the postings are not valid.
    ///
    /// # Errors
    ///
    /// - [`LedgerError::Empty`] if there are no postings.
    /// - [`LedgerError::Unbalanced`] if the amounts do not sum to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use ledger_core::money::Money;
    /// use ledger_core::transaction::{Posting, Transaction};
    ///
    /// let balanced = Transaction::new(vec![
    ///     Posting { account: "cash".into(), amount: Money(-1000) },
    ///     Posting { account: "etf:VWRL".into(), amount: Money(1000) },
    /// ]);
    /// assert!(balanced.is_ok());
    ///
    /// let unbalanced = Transaction::new(vec![
    ///     Posting { account: "cash".into(), amount: Money(-1000) },
    ///     Posting { account: "etf:VWRL".into(), amount: Money(900) },
    /// ]);
    /// assert!(unbalanced.is_err());
    /// ```
    pub fn new(postings: Vec<Posting>) -> Result<Transaction, LedgerError> {
        if postings.is_empty() {
            return Err(LedgerError::Empty);
        }
        // i128 can't overflow when adding i64 values, so the sum is exact.
        let sum: i128 = postings.iter().map(|p| i128::from(p.amount.0)).sum();
        if sum != 0 {
            return Err(LedgerError::Unbalanced { difference: sum });
        }
        Ok(Transaction { postings })
    }

    /// The postings, as a read-only slice.
    pub fn postings(&self) -> &[Posting] {
        &self.postings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn posting(account: &str, cents: i64) -> Posting {
        Posting {
            account: account.to_string(),
            amount: Money(cents),
        }
    }

    #[test]
    fn balanced_transaction_is_accepted() {
        let tx = Transaction::new(vec![posting("cash", -1000), posting("etf:VWRL", 1000)]);
        assert!(tx.is_ok());
    }

    #[test]
    fn unbalanced_transaction_is_rejected() {
        let tx = Transaction::new(vec![posting("cash", -1000), posting("etf:VWRL", 900)]);
        assert!(matches!(tx, Err(LedgerError::Unbalanced { .. })));
    }

    #[test]
    fn huge_amounts_are_summed_exactly() {
        let tx = Transaction::new(vec![posting("a", i64::MAX), posting("b", 1)]);
        assert!(matches!(tx, Err(LedgerError::Unbalanced { .. })));
    }

    #[test]
    fn empty_transaction_is_rejected() {
        let tx = Transaction::new(vec![]);
        assert!(matches!(tx, Err(LedgerError::Empty)));
    }
}
