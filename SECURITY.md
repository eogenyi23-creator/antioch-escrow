# Security Policy

## Status

This contract has **not been formally audited**. Do not use it with real funds on mainnet without an independent review.

## Reporting a vulnerability

Please report vulnerabilities **privately**. Do not open a public issue.

Use GitHub's private vulnerability reporting: open the **Security** tab of this repository and choose **Report a vulnerability**.

Include:
- a description of the problem and its impact
- steps or a test that reproduces it
- the commit or version you tested

## What to expect

- Acknowledgement within a few days
- A fix or a clear explanation as soon as we can manage, and credit to you if you want it
- Please allow time for a fix before disclosing publicly

## Scope

In scope: the contract in `contracts/escrow` and the client in `client/`.
Out of scope: Stellar network issues, wallet bugs, and issues in third-party dependencies (report those upstream).

See [docs/security.md](docs/security.md) for the threat model.
