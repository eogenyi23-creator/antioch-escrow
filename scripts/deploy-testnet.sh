#!/usr/bin/env bash
# scripts/deploy-testnet.sh — build and deploy the escrow contract to testnet.
#
# Usage:
#   ./scripts/deploy-testnet.sh [--source <identity>] [--network <name>]
#
# Options:
#   --source    Stellar CLI identity to sign the deploy transaction (default: "deployer")
#   --network   Stellar CLI network alias (default: "testnet")
#
# After running:
#   - Prints the deployed contract ID.
#   - You still need to call `init` before the escrow can accept funds.
#     See scripts/invoke-examples.sh for the init invocation example.
#
# Requirements:
#   - stellar CLI v25.2.0+ configured with a funded testnet identity.
#     Add one with: stellar keys generate --network testnet deployer
#     Fund it with:  stellar keys fund --network testnet deployer
#
# Note: This script does NOT call `init`. Run deploy first, copy the contract
# ID, then call init with your specific parameters.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

SOURCE="deployer"
NETWORK="testnet"

# Parse optional flags.
while [[ $# -gt 0 ]]; do
  case "$1" in
    --source) SOURCE="$2"; shift 2 ;;
    --network) NETWORK="$2"; shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 1 ;;
  esac
done

if ! command -v stellar &>/dev/null; then
  echo "ERROR: stellar CLI not found. Install it with:" >&2
  echo "  cargo install stellar-cli --features opt" >&2
  exit 1
fi

echo "==> Building contract..."
./scripts/build.sh

echo "==> Uploading WASM to $NETWORK..."
WASM_HASH=$(stellar contract upload \
  --wasm target/wasm32v1-none/release/escrow.wasm \
  --source "$SOURCE" \
  --network "$NETWORK")
echo "    WASM hash: $WASM_HASH"

echo "==> Deploying contract..."
CONTRACT_ID=$(stellar contract deploy \
  --wasm-hash "$WASM_HASH" \
  --source "$SOURCE" \
  --network "$NETWORK")

echo ""
echo "==> Deployed!"
echo "    Contract ID: $CONTRACT_ID"
echo ""
echo "Next step — initialise the escrow:"
echo "  ./scripts/invoke-examples.sh --contract $CONTRACT_ID --network $NETWORK"
