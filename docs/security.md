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
| Milestone released twice | Released flag checked and set before transfer; double release rejected |
| Donor refunds twice or takes more than their share | Refund recorded per donor; amount computed pro-rata with integer math rounded down |
| Rounding creates a payout larger than the balance | Division rounds down; invariant tests check that total paid out never exceeds total funded |
| Arithmetic overflow | Checked arithmetic on all `i128` operations |
| Re-initialization changes the parameters | `init` can be called once |
| Funds stranded if work never completes | Expiry ledger plus refund path |
| Storage archived on a long campaign | `maintain_ttl` extends entries |
| Fake or hostile token | Token address is fixed at init. Deployers should use only well-known SAC addresses |

## Out of scope

Wallet or frontend security, key management by users, off-chain evidence checking, and Stellar network or dependency vulnerabilities.

## Before mainnet

1. Independent audit
2. Testnet run with real approver workflows
3. Reviewed deployment and key-management plan for the approvers
