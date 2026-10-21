use std::sync::{Arc, Mutex};
use std::thread;

use ledger_core::idempotent::{IdempotentLedger, Outcome};
use ledger_core::money::Money;
use ledger_core::transaction::{Posting, Transaction};

fn buy() -> Transaction {
    Transaction::new(vec![
        Posting {
            account: "cash".to_string(),
            amount: Money(-1000),
        },
        Posting {
            account: "etf:VWRL".to_string(),
            amount: Money(1000),
        },
    ])
    .unwrap()
}

#[test]
fn concurrent_retries_apply_exactly_once() {
    let book = Arc::new(Mutex::new(IdempotentLedger::new()));

    let handles: Vec<_> = (0..100)
        .map(|_| {
            let book = Arc::clone(&book);
            thread::spawn(move || {
                let mut guard = book.lock().unwrap();
                guard.submit("key-1", buy()).unwrap()
            })
        })
        .collect();

    let outcomes: Vec<Outcome> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    let applied = outcomes.iter().filter(|o| **o == Outcome::Applied).count();
    assert_eq!(applied, 1);
    assert_eq!(book.lock().unwrap().ledger().balance("etf:VWRL").0, 1000);
}
