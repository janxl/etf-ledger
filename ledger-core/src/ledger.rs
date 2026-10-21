use std::collections::HashMap;

use crate::error::LedgerError;
use crate::money::Money;
use crate::transaction::Transaction;

/// Account balances, changed only by applying balanced transactions.
///
/// Because every transaction sums to zero, [`Ledger::total`] is always zero.
#[derive(Debug, Default)]
pub struct Ledger {
    balances: HashMap<String, i64>,
}

impl Ledger {
    /// An empty ledger.
    pub fn new() -> Ledger {
        Ledger::default()
    }

    /// The balance of an account in cents. Unknown accounts have a balance of zero.
    pub fn balance(&self, account: &str) -> Money {
        Money(self.balances.get(account).copied().unwrap_or(0))
    }

    /// Sum of every account. Must always be zero.
    pub fn total(&self) -> i128 {
        self.balances.values().map(|&v| i128::from(v)).sum()
    }

    /// Applies a transaction atomically: every posting is applied, or none is.
    ///
    /// # Errors
    ///
    /// [`LedgerError::Overflow`] if any account balance would overflow.
    pub fn apply(&mut self, tx: &Transaction) -> Result<(), LedgerError> {
        // Stage the new balances first, so a failure halfway changes nothing.
        let mut staged: HashMap<String, i64> = HashMap::new();

        for p in tx.postings() {
            let current = staged
                .get(&p.account)
                .or_else(|| self.balances.get(&p.account))
                .copied()
                .unwrap_or(0);
            let updated = current
                .checked_add(p.amount.0)
                .ok_or(LedgerError::Overflow)?;
            staged.insert(p.account.clone(), updated);
        }

        // Nothing failed: commit everything at once.
        self.balances.extend(staged);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn apply_moves_money_between_accounts() {
        let mut ledger = Ledger::new();
        ledger
            .apply(&tx(&[("cash", -1000), ("etf:VWRL", 1000)]))
            .unwrap();
        ledger
            .apply(&tx(&[("cash", -500), ("etf:VWRL", 500)]))
            .unwrap();

        assert_eq!(ledger.balance("cash").0, -1500);
        assert_eq!(ledger.balance("etf:VWRL").0, 1500);
        assert_eq!(ledger.total(), 0);
    }

    #[test]
    fn unknown_account_has_zero_balance() {
        assert_eq!(Ledger::new().balance("nobody").0, 0);
    }

    #[test]
    fn failed_apply_changes_nothing() {
        let mut ledger = Ledger::new();
        ledger
            .apply(&tx(&[("a", i64::MAX), ("b", -i64::MAX)]))
            .unwrap();

        // "c" is processed first and fine; then "a" overflows.
        let result = ledger.apply(&tx(&[("c", -1), ("a", 1)]));

        assert!(matches!(result, Err(LedgerError::Overflow)));
        assert_eq!(ledger.balance("c").0, 0); // c was not touched
        assert_eq!(ledger.balance("a").0, i64::MAX);
        assert_eq!(ledger.total(), 0);
    }
}
