# Security

**This contract has not been audited.** This document describes the intended security model and its limits. Report problems via [SECURITY.md](../SECURITY.md).

## Assets at risk

Tokens held by the escrow contract on behalf of donors.

## Trust assumptions

- The Stellar network and Soroban runtime behave correctly.
- The configured SAC token behaves as a standard token.
- At least M approvers are honest and keep their keys safe. This is the central trust assumption: the contract can enforce a threshold, but it cannot judge whether work was really done.

## Threats and mitigations

| Threat | Mitigation |
|---|---|
| One approver is compromised | Threshold of M approvers required; no single approver can release alone |
| M approvers collude or are all compromised | Not prevented. Choose independent approvers. Release goes only to the fixed beneficiary, never to an arbitrary address |
| Approver approves the same milestone twice to reach the threshold | Per-approver approval tracking; repeat approvals rejected |
| Milestone released twice | Released flag set **before** the token transfer; double release rejected |
| Donor refunds twice or takes more than their share | Refund recorded per donor; amount computed pro-rata with integer math rounded down |
| Rounding creates a payout larger than the balance | Division rounds down; invariant tests (randomized, multiple seeds) verify total paid out never exceeds total funded |
| Arithmetic overflow | Checked arithmetic on all `i128` operations |
| Re-initialization changes the parameters | `__constructor` runs atomically at deploy time; `AlreadyInitialized` guard prevents re-running |
| Front-run between deploy and init | Eliminated: `__constructor` makes deploy and init atomic — there is no separate init call to front-run |
| Over-funding / surplus tokens held by contract | `fund` rejects deposits where `funded_total + amount > sum(milestones)` (`FundingCapExceeded`); no surplus can accumulate |
| Re-entrancy during release | State (released flag and totals) is committed before the token transfer, so a re-entrant or reverted transfer cannot enable a second release |
| Funds stranded if work never completes | Expiry ledger plus refund path |
| Storage archived on a long campaign | `maintain_ttl` extends instance and milestone entries; `maintain_donor_ttl` extends per-donor entries |
| Fake or hostile token | Token address is fixed at construction. Deployers should use only well-known SAC addresses |

## Out of scope

Wallet or frontend security, key management by users, off-chain evidence checking, and Stellar network or dependency vulnerabilities.

## Before mainnet

1. Independent audit
2. Testnet run with real approver workflows
3. Reviewed deployment and key-management plan for the approvers
