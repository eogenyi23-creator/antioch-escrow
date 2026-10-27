# Design

## Purpose

Hold donations in a contract and release them to a beneficiary milestone by milestone, only when a threshold of independent approvers agrees. If the escrow expires, donors reclaim what was never released.

## Roles

| Role | Responsibility |
|---|---|
| Donor | Any address that funds the escrow. Authorizes each `fund` call. Can claim a refund after expiry. |
| Beneficiary | Fixed at construction. Receives released milestone amounts. |
| Approver | One of N fixed addresses. Approves milestones. |
| Token | Fixed Stellar Asset Contract (SAC) address, for example USDC. |

### Improvement over a single arbiter

Many escrows trust one arbiter to release funds. A compromised or careless arbiter can then release everything early. Antioch Escrow requires **M of N** approvers (for example 2 of 3, such as a pastor, a treasurer and an independent reviewer). No single approver can release funds alone.

## Money

All amounts are `i128` values in the token's smallest unit (7 decimals for USDC). The contract never parses decimal strings and never uses floats. The client's `toStroops` / `fromStroops` helpers enforce at most 7 decimal places.

## State machine

```text
Uninitialized
   | __constructor(approvers, threshold, beneficiary, token, milestones, expiry_ledger)
   v
Active  <-- fund / approve_milestone loop
   |  all milestones released          |  ledger >= expiry_ledger
   v                                   v
Completed                           Expired  (only refund works)
```

- The `__constructor` runs atomically at deploy time — deploy and initialization cannot be front-run because there is no separate `init` call. Beneficiary, token, approvers, threshold, milestones and expiry cannot change afterwards.
- In `Expired`, `fund` and `approve_milestone` are rejected.

## Functions

| Function | Caller | Rules |
|---|---|---|
| `__constructor` | deployer, once at deploy | threshold in 1..=N, no duplicate approvers, at least one milestone, each amount > 0, expiry strictly in the future (`> current ledger`) |
| `fund(donor, amount)` | donor (auth) | amount > 0, state Active, `funded_total + amount ≤ sum(milestones)` (funding cap), transfers token to the contract, records contribution |
| `approve_milestone(approver, index)` | approver (auth) | approver in set, valid index, not released, no repeat approval by the same approver; attempts release at threshold |
| `release_milestone(index)` | anyone (no auth) | threshold met, funds sufficient, in-order, not already released, not expired; pays the fixed beneficiary |
| `refund(donor)` | donor (auth) | only after expiry, once per donor |
| views | anyone | `state`, `funded_total`, `released_total`, `milestone`, `milestone_count`, `donor_contribution`, `approvals` |
| `maintain_ttl(start_index)` | anyone | extends storage TTL for instance, milestone released flags, and approval count entries |
| `maintain_donor_ttl(donor)` | anyone | extends storage TTL for a single donor's contribution and refund entries |

## Release rule

A milestone releases when its approvals reach the threshold **and** the contract holds enough unreleased funds to cover its amount.

If the threshold is reached but funds are short, the approvals stay recorded and the milestone remains pending. Once more funding arrives, a permissionless `release_milestone(index)` can release it. Anyone can call `release_milestone` — it only ever pays the fixed beneficiary, so it adds no trust. `approve_milestone` also attempts an automatic release when the threshold is crossed, so a manual call is only needed when funds were short at approval time.

### State before transfer

In `try_release`, the released flag and `released_total` are updated **before** the token transfer. This ensures that even if the transfer were to revert or a re-entrancy occurred, the milestone is already marked as released and cannot be double-released.

### In-order release

Milestones must release in order: milestone *n* can only be released after milestone *n−1* has been released (milestone 0 has no predecessor). This applies to both the automatic release inside `approve_milestone` and the manual `release_milestone` call. Sequential release makes the refund math and the public timeline simpler to reason about.

## Refund math

After expiry, each donor's refund is:

```
refund = contribution * (funded_total - released_total) / funded_total
```

- Integer division rounds down, so any rounding dust stays in the contract. The contract never pays out more than it holds.
- A donor can refund once. Track refunded donors so a second call fails.
- Compute with checked arithmetic. `contribution * unreleased` must not overflow `i128`.

## Funding cap

`fund` rejects any deposit that would push `funded_total` above the sum of all milestone amounts:

```
if funded_total + amount > sum(milestones) → Error::FundingCapExceeded
```

This guarantees the contract can never hold more tokens than are needed to fund all milestones. After all milestones are released the escrow enters `Completed` and `fund` is rejected entirely, so no surplus can accumulate.

## Errors

The contract uses a typed `Error` enum (see `contracts/escrow/src/error.rs`). Key error variants:

| Code | Name | When |
|---|---|---|
| 9 | `InsufficientFunds` | `release_milestone` — unreleased balance < milestone amount |
| 16 | `FundingCapExceeded` | `fund` — deposit would exceed total milestone amounts |
| 17 | `ThresholdNotMet` | `release_milestone` — fewer than `threshold` approvals recorded |
| 18 | `InvalidExpiry` | `__constructor` — `expiry_ledger` is not strictly in the future |

`ThresholdNotMet` is used (not `InsufficientFunds`) when the release fails due to insufficient approvals. `InvalidExpiry` is used (not `NotExpired`) for a bad expiry at construction time.

The TypeScript client provides `decodeError(code)` and `decodeErrorFromResult(error)` to map numeric codes back to readable names and messages.

## Events

Emitted on init, fund, approval, release and refund, so indexers and the app can follow state without reading storage.

## Storage and TTL

Persistent entries (milestones, approvals, contributions) can be archived if their TTL runs out.

- `maintain_ttl(start_index)` extends the instance storage and per-milestone entries (released flags and approval counts) starting from `start_index`. Each entry is only extended if it exists (`has()` check) and its TTL is below the threshold.
- `maintain_donor_ttl(donor)` extends a single donor's contribution and refund entries. Call this for each active donor.
- TTL constants: `TTL_THRESHOLD` = 17,280 ledgers (~1 day at ~5 s per ledger). `TTL_TARGET` = 518,400 ledgers (~30 days).
- Recommended: call `maintain_ttl` and `maintain_donor_ttl` for all donors monthly to keep a long-lived escrow from being archived.

## Known limitations

- Approvers are fixed at construction. Replacing a lost key requires a new escrow.
- If every approver becomes unavailable, milestones cannot release. Donors recover unreleased funds after expiry.
- The contract does not verify real-world work. Approvers are trusted to check evidence honestly.
- Rounding dust from pro-rata refunds stays in the contract; it is not recoverable.
- Unaudited. See [security.md](security.md).
