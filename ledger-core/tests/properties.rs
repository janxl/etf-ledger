use ledger_core::idempotent::{IdempotentLedger, Outcome};
use ledger_core::ledger::Ledger;
use ledger_core::money::Money;
use ledger_core::transaction::{Posting, Transaction};
use proptest::prelude::*;

const ACCOUNTS: [&str; 4] = ["cash", "etf:VWRL", "etf:IWDA", "fees"];

fn posting(account: usize, cents: i64) -> Posting {
    Posting {
        account: ACCOUNTS[account].to_string(),
        amount: Money(cents),
    }
}

/// Random postings, plus one final posting that makes them sum to zero.
fn balanced_tx() -> impl Strategy<Value = Transaction> {
    prop::collection::vec((0..ACCOUNTS.len(), -1_000_000i64..1_000_000), 1..8).prop_map(|raw| {
        let sum: i64 = raw.iter().map(|(_, cents)| cents).sum();
        let mut postings: Vec<Posting> = raw.iter().map(|&(a, c)| posting(a, c)).collect();
        postings.push(posting(0, -sum));
        Transaction::new(postings).unwrap()
    })
}

/// Pairs of +x / -x in a random order, over the full i64 range.
fn balanced_postings_any_order() -> impl Strategy<Value = Vec<Posting>> {
    prop::collection::vec(-i64::MAX..=i64::MAX, 1..4).prop_flat_map(|xs| {
        let postings: Vec<Posting> = xs
            .iter()
            .flat_map(|&x| [posting(0, x), posting(1, -x)])
            .collect();
        Just(postings).prop_shuffle()
    })
}

proptest! {
    #[test]
    fn total_is_always_zero(txs in prop::collection::vec(balanced_tx(), 0..20)) {
        let mut ledger = Ledger::new();
        for tx in &txs {
            ledger.apply(tx).unwrap();
        }
        prop_assert_eq!(ledger.total(), 0);
    }

    #[test]
    fn replaying_requests_never_changes_state(
        txs in prop::collection::vec(balanced_tx(), 1..10)
    ) {
        let mut book = IdempotentLedger::new();
        for (i, tx) in txs.iter().enumerate() {
            book.submit(&format!("key-{i}"), tx.clone()).unwrap();
        }
        let snapshot = |book: &IdempotentLedger| -> Vec<i64> {
            ACCOUNTS.iter().map(|a| book.ledger().balance(a).0).collect()
        };
        let before = snapshot(&book);

        for (i, tx) in txs.iter().enumerate() {
            let outcome = book.submit(&format!("key-{i}"), tx.clone()).unwrap();
            prop_assert_eq!(outcome, Outcome::Replayed);
        }

        prop_assert_eq!(before, snapshot(&book));
    }

    #[test]
    fn balanced_postings_are_always_accepted(postings in balanced_postings_any_order()) {
        prop_assert!(Transaction::new(postings).is_ok());
    }
}
