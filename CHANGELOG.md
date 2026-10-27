# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Milestone escrow contract: `init`, `fund`, `approve_milestone`, `release_milestone`, `refund`, views and `maintain_ttl`
- M-of-N approver threshold
- In-order milestone release (milestone *n* only after *n−1*)
- Pro-rata refunds after expiry
- TypeScript client for building unsigned transactions and reading state
- Testnet deploy and demo scripts
- Design and security documentation

---

## 2026-10-27 — contract fixes, TypeScript client, scripts

> Note: this project does not yet have PR numbers. PR numbers will be appended
> when pull requests are opened.

### Changes

**`contracts/escrow/src/events.rs`**
- Rewrote all five event emitters (`init`, `fund`, `approve`, `release`, `refund`)
  to use the `#[contractevent]` struct macro instead of the deprecated
  `env.events().publish()` API.

  **Why:** `soroban-sdk` 29 deprecates `Events::publish` and emits a clippy
  `-D warnings` error. The `#[contractevent]` macro includes event types in
  the contract interface spec so generated SDKs and indexers can understand them.

**`contracts/escrow/src/lib.rs`**
- Removed all unnecessary `as u32` casts on `soroban_sdk::Vec::len()` calls
  (in `init`, `approve_milestone`, `try_release`, `milestone`, `milestone_count`,
  `approvals`, and `maintain_ttl`).

  **Why:** `Vec::len()` in soroban-sdk already returns `u32`; the cast is
  redundant and was caught by `clippy::unnecessary_cast` under `-D warnings`.

**`contracts/escrow/src/storage.rs`**
- Removed `as u32` cast on loop variable `i` in `derive_state`.

  **Why:** Same reason as above — loop variable is already `u32`.

**`contracts/escrow/src/test.rs`**
- Removed unused `IntoVal` import.
- Renamed `contract_id` and `token_admin` fields in `TestSetup` to
  `_contract_id` and `_token_admin` to suppress dead-code warnings.

  **Why:** Both were caught by `clippy::unused_imports` and `dead_code` under
  `-D warnings`.

**`.gitignore`**
- Added `test_snapshots/` entry.

  **Why:** The Soroban test runner generates snapshot files in
  `contracts/escrow/test_snapshots/`. These are regenerated on every test run
  and should not be committed; committing them creates spurious diffs.

- Removed the committed `contracts/escrow/test_snapshots/` directory.

  **Why:** The directory was already tracked in git before the `.gitignore`
  entry was added.

**`client/` (new)**
- Added `client/package.json`, `client/tsconfig.json`, `client/jest.config.js`,
  `client/eslint.config.js`.
- Added `client/src/amounts.ts`: `toStroops` / `fromStroops` helpers with strict
  7-decimal validation. No floats; BigInt arithmetic throughout.
- Added `client/src/types.ts`: all exported TypeScript interfaces and types.
- Added `client/src/client.ts`: `EscrowClient` class with:
  - `buildFund`, `buildApproveMilestone`, `buildReleaseMilestone`, `buildRefund`
    — each returns an unsigned base64 XDR envelope; no private key handling.
  - `readState`, `readFundedTotal`, `readReleasedTotal`, `readMilestone`,
    `readMilestoneCount`, `readApprovals`, `readDonorContribution`,
    `readSnapshot` — simulate view functions via Soroban RPC.
- Added `client/src/index.ts`: barrel re-export.
- Added `client/src/__tests__/amounts.test.ts`: 24 tests for `toStroops` /
  `fromStroops` (correct conversions, edge cases, error paths, round-trips).
- Added `client/src/__tests__/client.test.ts`: 23 tests for `build*` methods
  (decode XDR, verify function name and argument count, no RPC needed).

  **Why:** The spec requires a TypeScript client that builds unsigned XDR and
  never holds keys. Tests run without network access by constructing transactions
  locally and decoding them with stellar-sdk.

  **Known limitation:** `@stellar/stellar-sdk` 17 has pure-ESM transitive
  dependencies (`uint8array-extras`, `@noble/hashes`, `@exodus/bytes`). Tests
  require `node --experimental-vm-modules` (set in the `test` script in
  `package.json`). This is documented in the README and in `jest.config.js`.
  ESLint 10 uses the new flat config format (`eslint.config.js`), not
  `.eslintrc.json`.

**`scripts/build.sh` (new)**
- Builds the escrow contract WASM. Uses `stellar contract build` if the stellar
  CLI is installed; falls back to
  `SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 cargo build --target
  wasm32v1-none --release` otherwise.

  **Why:** `soroban-sdk` 29 requires either the stellar CLI or this env var to
  build; the fallback makes the script usable in environments without the CLI.

**`scripts/deploy-testnet.sh` (new)**
- Uploads the WASM and deploys a new contract instance on testnet; prints the
  contract ID.

**`scripts/invoke-examples.sh` (new)**
- Calls `init` with a sample 2-of-3 approver, 2-milestone escrow. Intended as
  a reference for the CLI invocation syntax.

**`scripts/demo.sh` (new)**
- Full end-to-end demo: build → deploy → init → fund → approve (x2) → release
  → read state. Requires funded testnet identities named `deployer`,
  `approver1`, `approver2`, `approver3`, `donor`, `beneficiary`.

### Test results (as of this entry)

```
cargo fmt --all -- --check : pass
cargo clippy --workspace --all-targets -- -D warnings : 0 errors, 0 warnings
cargo test --workspace : 28 tests, 0 failures
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 cargo build --target wasm32v1-none --release : escrow.wasm produced
cd client && npm run lint : 0 warnings
cd client && npm run build : success (tsc)
cd client && npm test : 47 tests, 0 failures
```

### Known limitations and deliberately excluded items

- `stellar contract build` (stellar CLI) is not available in this build
  environment; WASM build uses the fallback env var. The CI workflow uses the
  CLI via `dtolnay/rust-toolchain` + `cargo install stellar-cli`, so CI will
  use the preferred path.
- Read helpers in `EscrowClient` require a live Soroban RPC endpoint; they are
  not tested against a real network in this test suite.
- Scripts require a funded testnet identity and the stellar CLI; they are not
  run in CI.

