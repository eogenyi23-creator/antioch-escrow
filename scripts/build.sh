#!/usr/bin/env bash
# scripts/build.sh — compile the escrow contract to WASM.
#
# Usage:
#   ./scripts/build.sh
#
# Output:
#   target/wasm32v1-none/release/escrow.wasm
#
# Requirements:
#   - Rust (stable) with the wasm32v1-none target installed:
#       rustup target add wasm32v1-none
#   - stellar CLI v25.2.0+ (preferred):
#       cargo install stellar-cli --features opt
#     OR set SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 to build
#     without the stellar CLI (useful in CI environments where the CLI is
#     not available).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "==> Building escrow contract to WASM..."

if command -v stellar &>/dev/null; then
  # Preferred: use the stellar CLI which sets the required env var automatically.
  stellar contract build
else
  # Fallback: build directly with cargo.
  # SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 tells soroban-sdk 29+
  # that the build system supports spec shaking (required when not using the
  # stellar CLI).
  echo "  (stellar CLI not found; using cargo with spec-shaking env var)"
  SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 \
    cargo build --target wasm32v1-none --release
fi

WASM="target/wasm32v1-none/release/escrow.wasm"
if [[ ! -f "$WASM" ]]; then
  echo "ERROR: WASM artifact not found at $WASM" >&2
  exit 1
fi

SIZE=$(du -h "$WASM" | cut -f1)
echo "==> Built: $WASM ($SIZE)"
