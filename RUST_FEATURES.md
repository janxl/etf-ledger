# Rust features in etf-ledger

A plain-language tour of the Rust ideas this demo uses, and why they matter for handling money.
Setup and running: see the [README](README.md).

## How the code is organised

- `ledger-core/` holds the money logic. It has no web code, so it is easy to test.
- `ledger-api/` is a thin web server on top of it.

## The features, in plain words

**1. Types that prevent mix-ups** (`money.rs`)
Money is a small wrapper type around a whole number of cents. It can't be confused with any other number, and it never uses decimals, so there are no rounding errors.

**2. Invalid things can't be built** (`transaction.rs`)
A transaction must add up to zero (what goes out equals what comes in). The only way to create one checks this. If you have a `Transaction`, it is already valid.

**3. Errors are values, not surprises** (`error.rs`)
A function that can fail says so in its return type (`Result`), and the compiler makes you handle it. There are no hidden exceptions.

**4. The compiler checks you handled every case** (`ledger-api`)
Each kind of error is mapped to an HTTP status with a `match`. Add a new error and the code won't compile until you decide what it should return.

**5. Ownership: safe changes** (`ledger.rs`)
Rust tracks who owns each piece of data. Applying a transaction first works out all the new balances, and only then saves them. If anything fails, nothing changes. Outside code gets a read-only view, so it can't skip the rules.

**6. Overflow is an error, not a silent bug** (`ledger.rs`)
Normal maths can quietly wrap around when numbers get too big. Here, "checked" maths returns an error instead.

**7. Safe concurrency** (`tests/concurrency.rs`)
Many requests can arrive at once. A lock (`Mutex`) lets one at a time touch the data, and `Arc` lets them all share it. The test sends 100 identical requests at once and the money moves exactly once. Sharing data unsafely between threads doesn't compile.

**8. Tests that invent their own cases** (`tests/properties.rs`)
Property tests state a rule ("totals always add up to zero") and check it against thousands of random inputs. This found a real bug in our first version that hand-written examples missed.

**9. Web input is checked by types** (`ledger-api`)
The server describes what a request should look like as a type. Bad JSON, missing fields and wrong types are rejected before our code runs. The API is tested in-process, with no network port (`ledger-api/tests/api.rs`).

**10. Two crates in one workspace** (root `Cargo.toml`)
The money logic and the web layer are separate packages that build together.

**11. Documentation that is tested** (`///` examples)
Code examples inside doc comments are compiled and run by `cargo test`, so the docs can't drift out of date.

## Why Rust fits this problem

- **Money bugs are costly.** Many mistakes become compile errors instead of production incidents.
- **Retries and parallel requests are normal.** Rust stops data races at compile time.
- **Fast and predictable.** No garbage collector, so no random pauses.
- **Easy to change safely.** The compiler lists everything affected by a change.
- **Simple to deploy.** One small program file.

## Honest downsides

- Harder to learn (ownership takes time).
- Slower to compile than Go or Python.
- Fewer finance-specific libraries than Java or .NET.

## What this demo leaves out

- Data is in memory only. A restart loses everything. Production would use a database (for example Postgres) with a unique constraint on the idempotency key.
- One global lock. Production would lock per account or let the database do it.
- No login, permissions, audit trail or logging yet.
