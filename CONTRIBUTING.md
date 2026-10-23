# Contributing to Antioch Escrow

Thanks for helping. This is a smart contract that may hold other people's money, so we favor small, well-tested changes over large ones.

## Before you start

- Open an issue (or comment on an existing one) before starting substantial work, so we can agree on the approach.
- Security problems go through [SECURITY.md](SECURITY.md), not public issues.

## Setup

```bash
git clone https://github.com/eogenyi23-creator/antioch-escrow.git
cd antioch-escrow
rustup target add wasm32v1-none
cargo test
cd client && npm install && npm test
```

You will need Rust (stable), the [Stellar CLI](https://developers.stellar.org/docs/tools/cli) and Node.js 20+.

## Workflow

1. Fork and create a branch from `main` (`fix/refund-rounding`, `feat/ttl-helper`).
2. Make your change with tests.
3. Run the checks below.
4. Open a pull request against `main` and fill in the template.

## Checks that must pass

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --target wasm32v1-none --release
cd client && npm run lint && npm run build && npm test
```

CI runs the same commands.

## Contract coding standards

- Money is `i128` in the token's smallest unit. No floats, no decimal parsing.
- Use checked arithmetic. Never allow overflow or underflow to wrap silently.
- Every state-changing function calls `require_auth()` on the right address first.
- Add a typed error for every failure path. Avoid `panic!` and `unwrap()` in contract code.
- Every bug fix includes a test that fails without the fix.
- Changing the public interface requires updating `docs/design.md`, the README table and the client.

## Commit messages

Short, imperative, one idea per commit: `Reject duplicate approvals`, `Fix refund rounding for dust`.

## Code of conduct

Be respectful and assume good faith. Harassment or abuse is not tolerated.
