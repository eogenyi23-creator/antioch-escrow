# Antioch Escrow — Specification

> Working spec for the milestone-escrow contract and TypeScript client.
> This document is the source of truth for implementation. Where it conflicts
> with earlier drafts of `docs/design.md`, this spec wins.

---

## Repo audit

### File tree (as found)

```
antioch-escrow/
  .git/                        (git metadata — not tracked content)
  CHANGELOG.md
  CONTRIBUTING.md
  LICENSE                      (MIT, © 2026 eogenyi23-creator)
  README.md
  SECURITY.md
  docs/
    design.md
    security.md
```

### Comparison against expected set

| Item | Status | Notes |
|---|---|---|
| `README.md` | Present | Complete; describes contract, features, interface table, quick start. Already includes `approve_milestone` but not `release_milestone`. |
| `LICENSE` | Present | MIT. |
| `CHANGELOG.md` | Present | [Unreleased] section lists contract, client, scripts. Does not mention `release_milestone` or in-order release. |
| `CONTRIBUTING.md` | Present | Setup, workflow, checks, coding standards. References `cargo build --target wasm32v1-none --release`. |
| `SECURITY.md` | Present | Reporting policy and scope. |
| `.gitignore` | **Missing → created** | Added in Step 0. Ignores `target/`, `node_modules/`, `client/dist/`, `*.wasm`, `.env`, `.env.*` (except `.env.example`), `.stellar/`, `.soroban/`. |
| `docs/design.md` | Present | Roles, state machine, functions, release rule, refund math, events, storage/TTL, limitations. Lists `release_milestone` and in-order release as **open decisions** — must be settled (see below). |
| `docs/security.md` | Present | Threat model, mitigations, out-of-scope, before-mainnet checklist. No conflicts with this spec. |
| `.github/workflows/ci.yml` | **Missing** | To be created. |
| `.github/PULL_REQUEST_TEMPLATE.md` | **Missing** | To be created. |
| `.github/ISSUE_TEMPLATE/bug_report.md` | **Missing** | To be created. |
| `.github/ISSUE_TEMPLATE/feature_request.md` | **Missing** | To be created. |
| `contracts/escrow/` | **Missing** | To be built: `src/lib.rs`, `error.rs`, `storage.rs`, `events.rs`, `test.rs`, `Cargo.toml`. |
| `client/` | **Missing** | To be built: TypeScript client + tests. |
| `scripts/` | **Missing** | To be built: build, deploy-testnet, invoke examples, demo. |

### Conflicts between existing files and this spec

1. **`docs/design.md` — release rule (open decision):** The design doc lists `release_milestone(index)` as a recommended but unsettled open decision. **This spec finalizes it:** `release_milestone(index)` is a required public function, callable by anyone without auth. It releases a milestone whose approvals already meet the threshold and whose funds are now sufficient, respecting the in-order rule. `docs/design.md` must be edited to remove the "Open decision" framing and document the final behavior.

2. **`docs/design.md` — in-order release (open decision):** The design doc lists sequential release as a recommended but unsettled open decision. **This spec finalizes it:** milestones must release in order — milestone *n* can only be released after milestone *n−1* has been released. `docs/design.md` must be edited accordingly.

3. **`README.md` — contract interface table:** The table lists `approve_milestone` but not `release_milestone`. It must be updated to add `release_milestone(index)`.

4. **`CHANGELOG.md` — Added list:** Does not mention `release_milestone` or in-order release. Must be updated.

No other conflicts were found. `docs/security.md`, `SECURITY.md`, `CONTRIBUTING.md`, `LICENSE`, and `README.md` (apart from the interface table) are consistent with this spec.

---

## Requirements

### R1 — Contract: milestone escrow on Soroban

A Soroban smart contract (`contracts/escrow/`) that holds donations in a Stellar Asset Contract (SAC) token and releases them to a beneficiary milestone by milestone, only when a threshold of independent approvers agrees. If the escrow expires, donors reclaim their pro-rata share of unreleased funds.

- **Rust**, latest stable `soroban-sdk`, `#![no_std]`, workspace layout under `contracts/`.
- All money is `i128` in the token's smallest unit (7 decimals for USDC). Never parse decimals or use floats in the contract.
- Typed `Error` enum for every failure path. No `panic!` or `unwrap()` in contract code.
- Checked arithmetic on all `i128` operations.
- Every state-changing function calls `require_auth()` on the right address first.

### R2 — Roles

| Role | Definition |
|---|---|
| Donor | Any address that funds the escrow. Authorizes each `fund` call. Can claim a refund after expiry. |
| Beneficiary | Fixed at `init`. Receives released milestone amounts. |
| Approver | One of N fixed addresses. Approves milestones. |
| Token | Fixed SAC address, for example USDC. |

### R3 — State machine

```
Uninitialized → Active → (Completed | Expired)
```

- `init` transitions Uninitialized → Active. Callable once.
- `Completed`: all milestones have been released.
- `Expired`: the expiry ledger has passed. After expiry, `fund` and `approve_milestone` are rejected; only `refund` works.

### R4 — Functions

#### init(approvers: Vec<Address>, threshold: u32, beneficiary: Address, token: Address, milestones: Vec<i128>, expiry_ledger: u32)

- Callable once (deployer).
- Validate: `threshold` between 1 and N (inclusive), no duplicate approvers, at least one milestone, every amount > 0, `expiry_ledger` in the future (greater than the current ledger).
- All parameters are fixed afterwards; none can change.
- Emits an init event.

#### fund(donor: Address, amount: i128)

- `donor.require_auth()`.
- `amount` must be > 0.
- Reject when Expired or Completed.
- Transfers `amount` of the token from `donor` to the contract via the SAC `transfer` function.
- Records the donor's contribution (accumulates across calls) and the funded total.
- Emits a fund event.

#### approve_milestone(approver: Address, index: u32)

- `approver.require_auth()`.
- The approver must be in the fixed set.
- The `index` must be valid (0 ≤ index < milestone_count).
- The milestone must not already be released.
- The same approver cannot approve the same milestone twice.
- Reject when Expired.
- When approvals for the milestone reach the threshold, **attempt** to release automatically (see release rule). If funds are short, do not fail the approval — keep it recorded and leave the milestone pending.
- Emits an approval event. Emits a release event if the milestone releases.

#### release_milestone(index: u32)

- Callable by anyone. No auth required.
- Releases a milestone whose approvals already meet the threshold and whose unreleased balance is now sufficient, respecting the in-order rule.
- Reject if: already released, threshold not met, funds insufficient, out of order (previous milestone not yet released), or expired.
- Transfers the milestone amount to the fixed beneficiary.
- Updates `released_total`. Sets the milestone's released flag.
- Emits a release event.
- This only ever pays the fixed beneficiary, so it adds no trust.

#### refund(donor: Address) → i128

- `donor.require_auth()`.
- Only after expiry (state is Expired).
- Returns the donor's pro-rata share of the unreleased balance:

  ```
  refund = contribution × (funded_total − released_total) ÷ funded_total
  ```

  Integer division rounds down. Rounding dust stays in the contract.

- Track refunded amounts so a donor cannot refund twice. A donor with zero contribution or who already refunded gets `NothingToRefund`.
- Emits a refund event with the amount.

#### Views (read-only, anyone)

| Function | Returns |
|---|---|
| `state()` | Current state: `Uninitialized`, `Active`, `Completed`, or `Expired` |
| `funded_total()` | Total tokens ever funded |
| `released_total()` | Total tokens ever released to beneficiary |
| `milestone(index)` | Milestone amount and released flag |
| `milestone_count()` | Number of milestones |
| `approvals(index)` | Number of approvals recorded for milestone `index` |
| `donor_contribution(donor)` | A donor's total contribution |

#### maintain_ttl(start_index: u32)

- Callable by anyone. No auth.
- Extends storage TTL for persistent entries starting from `start_index` so long-lived escrows are not archived.
- Iterates over milestone and approval entries, bumping their TTL.

### R5 — In-order release rule

Milestones must release in order: milestone *n* can only be released after milestone *n−1* has been released. Milestone 0 has no predecessor. This applies to both automatic release (inside `approve_milestone`) and manual release (`release_milestone`).

### R6 — Events

Emitted on: init, fund, milestone approval, milestone release, refund. Topics are indexer-friendly (contract address, event type, relevant indices/addresses).

### R7 — Errors

Typed `Error` enum:

- `NotInitialized`
- `AlreadyInitialized`
- `Unauthorized`
- `InvalidThreshold`
- `DuplicateApprover`
- `InvalidMilestone`
- `AlreadyReleased`
- `AlreadyApproved`
- `InsufficientFunds`
- `Expired`
- `NotExpired`
- `NothingToRefund`
- `AmountMustBePositive`
- `OutOfOrder`
- `Overflow`

### R8 — Tests (contract)

Unit tests with the Soroban test environment and a mock SAC token:

1. `init` validation: every invalid case (threshold 0, threshold > N, duplicate approver, no milestones, amount ≤ 0, expiry not in future, double init).
2. Funding by multiple donors; funding after expiry rejected; funding after completed rejected.
3. Threshold logic: 1 of 3 does not release; 2 of 3 releases; duplicate approval rejected; non-approver rejected.
4. Release blocked by insufficient funds, then succeeds via `release_milestone` after more funding.
5. `release_milestone` rejected when: threshold not met, already released, out of order, expired.
6. Milestones must release in order (cannot skip ahead).
7. Double release impossible (flag checked).
8. Refund before expiry rejected; refund after expiry pro-rata across multiple donors; double refund rejected.
9. Invariant property test (randomized): contract token balance is never negative and total paid out never exceeds total funded.
10. Completed state after all milestones released.

### R9 — TypeScript client

Package under `client/`. Uses `@stellar/stellar-sdk`. Builds unsigned transaction XDR for `fund`, `approveMilestone`, `releaseMilestone`, `refund`. Read helpers use Soroban RPC simulation. Never holds keys.

- Amount helpers: `toStroops(decimalString)` and `fromStroops(int128String)` with strict 7-decimal validation.
- Export types.
- Unit-test the helpers and XDR building.

### R10 — Scripts

- `scripts/build.sh` — build wasm.
- `scripts/deploy-testnet.sh` — build, deploy to testnet, print contract ID.
- `scripts/invoke-examples.sh` — init an example escrow.
- `scripts/demo.sh` — full demo: init, fund, approve twice, release. Print contract ID.

### R11 — CI

GitHub Actions workflow (`.github/workflows/ci.yml`):
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- Contract build to wasm (`cargo build --target wasm32v1-none --release` or `stellar contract build`)
- Client lint + build + test (`cd client && npm run lint && npm run build && npm test`)

### R12 — Docs

- `docs/design.md`: roles, state machine, threshold approval, in-order release, `release_milestone`, refund math, known limitations. Edit existing file; do not overwrite.
- `docs/security.md`: threat model and out-of-scope. Already present and consistent.
- `README.md`: one-line purpose, Mermaid architecture diagram, quick start, testnet contract ID (placeholder), usage example, security notes. Edit to add `release_milestone` to interface table.
- `CHANGELOG.md`: update [Unreleased] Added list.

---

## Design

### Money

All amounts are `i128` in the token's smallest unit (7 decimals for USDC). The contract never parses decimal strings and never uses floats. The client's `toStroops` / `fromStroops` helpers enforce at most 7 decimal places.

### Storage layout

Persistent storage keys (instance and persistent):

| Key | Type | Notes |
|---|---|---|
| `INIT` | `bool` | Whether `init` has run |
| `APPROVERS` | `Vec<Address>` | Fixed set |
| `THRESHOLD` | `u32` | M of N |
| `BENEFICIARY` | `Address` | Fixed |
| `TOKEN` | `Address` | Fixed SAC address |
| `MILESTONES` | `Vec<i128>` | Fixed amounts |
| `EXPIRY_LEDGER` | `u32` | After this ledger, Expired |
| `FUNDED_TOTAL` | `i128` | Accumulated |
| `RELEASED_TOTAL` | `i128` | Accumulated |
| `MILESTONE_RELEASED(index)` | `bool` | Per-milestone flag |
| `MILESTONE_APPROVALS(index, approver)` | `bool` | Per-approver per-milestone flag |
| `APPROVAL_COUNT(index)` | `u32` | Count of approvals for milestone |
| `DONOR_CONTRIBUTION(donor)` | `i128` | Per-donor total |
| `DONOR_REFUNDED(donor)` | `bool` | Whether donor has refunded |

### State derivation

The contract state is derived, not stored as a single field:

- `Uninitialized`: `INIT` is false.
- `Completed`: all milestones have `MILESTONE_RELEASED == true`.
- `Expired`: current ledger ≥ `EXPIRY_LEDGER` (and not all milestones released — though if all are released, Completed takes precedence since there is nothing to refund).
- `Active`: initialized, not expired, not all milestones released.

Precedence: Uninitialized > Completed > Expired > Active.

### Threshold approval

Each approver can approve each milestone at most once. `APPROVAL_COUNT(index)` tracks the count. When it reaches `THRESHOLD`, the milestone is eligible for release (pending funds and in-order constraint).

### In-order release

`release_milestone(index)` and the automatic release inside `approve_milestone` both check: if `index > 0`, milestone `index − 1` must already be released. This prevents skipping ahead and makes the refund math and public timeline simpler.

### Release flow

1. Check milestone not already released.
2. Check `index == 0` or milestone `index − 1` is released (in-order).
3. Check `APPROVAL_COUNT(index) >= THRESHOLD`.
4. Check unreleased balance (`FUNDED_TOTAL − RELEASED_TOTAL >= milestone_amount`).
5. Transfer `milestone_amount` to beneficiary via SAC `transfer`.
6. Set `MILESTONE_RELEASED(index) = true`.
7. `RELEASED_TOTAL += milestone_amount` (checked arithmetic).
8. Emit release event.

### Refund math

```
refund = contribution × (funded_total − released_total) / funded_total
```

- Integer division rounds down. Rounding dust stays in the contract.
- `contribution * unreleased` computed with checked arithmetic (must not overflow `i128`).
- A donor can refund once. `DONOR_REFUNDED(donor)` is set after refund.
- `NothingToRefund` if contribution is 0 or already refunded.

### Events

| Event | Topics | Data |
|---|---|---|
| `Init` | contract, "init" | beneficiary, token, threshold, milestone_count |
| `Fund` | contract, "fund", donor | amount |
| `Approve` | contract, "approve", approver, index | (none or approval count) |
| `Release` | contract, "release", index | amount, beneficiary |
| `Refund` | contract, "refund", donor | amount |

### TTL

Persistent entries can be archived if their TTL runs out. `maintain_ttl(start_index)` extends entries. The contract bumps TTL for milestone data, approval data, and donor data.

---

## Tasks

Implementation order — contract and tests before the client:

1. **[Done]** Repo audit
2. **[Done]** Create `.gitignore`
3. **[This file]** Write spec (requirements, design, tasks)
4. Edit `docs/design.md` — settle open decisions: add `release_milestone(index)` + in-order release
5. Edit `README.md` + `CHANGELOG.md` — add `release_milestone` and in-order release
6. Create `.github/` files — `ci.yml`, `PULL_REQUEST_TEMPLATE.md`, `ISSUE_TEMPLATE/bug_report.md`, `ISSUE_TEMPLATE/feature_request.md`
7. Implement Rust workspace + `contracts/escrow` — `lib.rs`, `error.rs`, `storage.rs`, `events.rs`, `test.rs`
8. Run `cargo test`, `cargo fmt`, `cargo clippy`, wasm build
9. Implement TypeScript client (`client/`) — amount helpers, XDR builders, read helpers, tests
10. Implement `scripts/` — build, deploy-testnet, invoke examples, demo
11. Wire up CI and run full verification — cargo fmt/clippy/test, wasm build, client lint/build/test
