## Summary

<!-- Brief description of what this PR changes and why. -->

## Changes

- [ ] Contract (`contracts/escrow/`)
- [ ] Client (`client/`)
- [ ] Scripts (`scripts/`)
- [ ] Docs (`README.md`, `docs/`, `CHANGELOG.md`)
- [ ] CI / `.github/`
- [ ] Other

## Testing

<!-- What did you test? How can a reviewer reproduce it? -->

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] `cargo build --target wasm32v1-none --release` passes
- [ ] `cd client && npm run lint && npm run build && npm test` passes

## Interface changes

<!-- If you changed the public contract interface, did you update docs/design.md, the README table, and the client? -->

- [ ] Not applicable
- [ ] Updated `docs/design.md`
- [ ] Updated `README.md` interface table
- [ ] Updated client
- [ ] Updated `CHANGELOG.md`

## Security

<!-- Does this change how funds are handled, who can authorize, or any access control? If so, explain why it is safe. -->

## Checklist

- [ ] Code is commented where logic is non-obvious
- [ ] No `panic!` or `unwrap()` in contract code
- [ ] Money is `i128`; no floats, no decimal parsing in the contract
- [ ] Every state-changing function calls `require_auth()` on the right address
