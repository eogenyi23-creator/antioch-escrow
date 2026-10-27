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
#   - The contract is deployed WITHOUT constructor args. To initialise it,
#     pass constructor args at deploy time using the stellar CLI:
#       stellar contract deploy --wasm-hash <HASH> --source <ID> --network <NET> \
#         -- __constructor --approvers '[...]' --threshold 2 \
#         --beneficiary <ADDR> --token <SAC_CONTRACT_ID> \
#         --milestones '[...]' --expiry_ledger <N>
#     Or use scripts/invoke-examples.sh which shows the constructor invocation.
#
# Requirements:
#   - stellar CLI v25.2.0+ configured with a funded testnet identity.
#     Add one with: stellar keys generate --network testnet deployer
#     Fund it with:  stellar keys fund --network testnet deployer

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
echo "Next step — initialise the escrow with constructor args:"
echo "  stellar contract deploy --wasm-hash $WASM_HASH --source $SOURCE --network $NETWORK \\"
echo "    -- __constructor --approvers '[...]' --threshold 2 \\"
echo "    --beneficiary <ADDR> --token <SAC_CONTRACT_ID> \\"
echo "    --milestones '[...]' --expiry_ledger <N>"
echo ""
echo "  Or use scripts/invoke-examples.sh --contract \$CONTRACT_ID --network $NETWORK"
