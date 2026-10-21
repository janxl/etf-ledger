# etf-ledger

[![CI](https://github.com/janxl/etf-ledger/actions/workflows/ci.yml/badge.svg)](https://github.com/janxl/etf-ledger/actions/workflows/ci.yml)

A small, in-memory **double-entry ledger service** with **idempotent request handling**, written in Rust.
It is a compact demo of the core an investment platform needs: money that is never created or lost,
and requests that are safe to retry.

- **Balanced by construction.** A transaction's postings must sum to zero; an unbalanced one cannot be built.
- **All-or-nothing.** Applying a transaction changes every account or none.
- **Idempotent.** Retrying a request with the same `Idempotency-Key` never applies it twice, even under concurrency.

See [RUST_FEATURES.md](RUST_FEATURES.md) for the Rust features it showcases and why Rust fits this problem.

## Install

1. Install Rust with [rustup](https://rustup.rs) (Linux, macOS, WSL2):

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```

   On Windows, use WSL2 (recommended), or install rustup together with the Visual Studio Build Tools.
   A C linker (`gcc` or `clang`) is required on Linux.

2. Get the code and build it:

   ```bash
   git clone https://github.com/janxl/etf-ledger.git
   cd etf-ledger
   cargo build
   ```

## Run

```bash
cargo run -p ledger-api      # listens on 127.0.0.1:3000
```

```bash
curl -i -X POST localhost:3000/transactions \
  -H 'Content-Type: application/json' -H 'Idempotency-Key: k1' \
  -d '{"postings":[{"account":"cash","cents":-1000},{"account":"etf:VWRL","cents":1000}]}'
# 201 {"outcome":"applied"}   (send it again: 200 {"outcome":"replayed"})

curl localhost:3000/accounts/etf:VWRL/balance
# {"account":"etf:VWRL","cents":1000}
```

## Test

```bash
cargo test
```

This runs the unit tests, the examples in the documentation, the concurrency tests
(many parallel requests with one idempotency key, applied exactly once), HTTP-level tests of the API
(run in-process, no network port needed), and the property-based tests.

CI also checks formatting (`cargo fmt --check`) and lints (`cargo clippy -- -D warnings`).

## License

MIT, see [LICENSE](LICENSE).
