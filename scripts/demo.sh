#!/usr/bin/env bash
# scripts/demo.sh — end-to-end demo: deploy, init, fund, approve twice, release.
#
# Usage:
#   ./scripts/demo.sh [--network <name>]
#
# What it does:
#   1. Builds the contract WASM.
#   2. Deploys it to testnet.
#   3. Calls init with a 1-milestone, 2-of-3 approver escrow.
#   4. Funds the escrow from a demo donor.
#   5. Approver 1 approves milestone 0.
#   6. Approver 2 approves milestone 0 (threshold reached → auto-release).
#   7. Reads state and prints a summary.
#
# Requirements:
#   - stellar CLI v25.2.0+ with the following funded testnet identities:
#       deployer, approver1, approver2, approver3, donor, beneficiary
#     Create them with:
#       for id in deployer approver1 approver2 approver3 donor beneficiary; do
#         stellar keys generate --network testnet $id
#         stellar keys fund --network testnet $id
#       done
#   - USDC testnet SAC address set in USDC_TOKEN below (or override with env var).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

NETWORK="${NETWORK:-testnet}"

# USDC on testnet — override with USDC_TOKEN env var if needed.
USDC_TOKEN="${USDC_TOKEN:-GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5}"

# Fund amount: 500 USDC in stroops.
FUND_AMOUNT=5000000000

# Milestone: 500 USDC in stroops.
MILESTONE_AMOUNT=5000000000

if ! command -v stellar &>/dev/null; then
  echo "ERROR: stellar CLI not found." >&2
  exit 1
fi

# Helper: resolve identity to public key.
addr() { stellar keys address "$1" --network "$NETWORK"; }

echo "=========================================="
echo " Antioch Escrow — end-to-end demo"
echo " Network: $NETWORK"
echo "=========================================="
echo ""

# 1. Build
echo "[1/7] Building contract..."
./scripts/build.sh

# 2. Deploy
echo ""
echo "[2/7] Deploying contract..."
WASM_HASH=$(stellar contract upload \
  --wasm target/wasm32v1-none/release/escrow.wasm \
  --source deployer \
  --network "$NETWORK")

CONTRACT_ID=$(stellar contract deploy \
  --wasm-hash "$WASM_HASH" \
  --source deployer \
  --network "$NETWORK")

echo "  Contract ID: $CONTRACT_ID"

# 3. Init
echo ""
echo "[3/7] Initialising escrow..."

APPROVER1=$(addr approver1)
APPROVER2=$(addr approver2)
APPROVER3=$(addr approver3)
BENEFICIARY=$(addr beneficiary)
DONOR=$(addr donor)

CURRENT_LEDGER=$(stellar ledger --network "$NETWORK" | jq -r '.sequence')
# Expiry: 10 minutes from now (~120 ledgers at ~5s each).
EXPIRY=$(( CURRENT_LEDGER + 120 ))

stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source deployer \
  --network "$NETWORK" \
  -- init \
  --approvers "[$APPROVER1,$APPROVER2,$APPROVER3]" \
  --threshold 2 \
  --beneficiary "$BENEFICIARY" \
  --token "$USDC_TOKEN" \
  --milestones "[$MILESTONE_AMOUNT]" \
  --expiry_ledger "$EXPIRY"

echo "  Initialised with 1 milestone of $MILESTONE_AMOUNT stroops (2-of-3 approval)."

# 4. Fund
echo ""
echo "[4/7] Funding escrow ($FUND_AMOUNT stroops from donor)..."
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source donor \
  --network "$NETWORK" \
  -- fund \
  --donor "$DONOR" \
  --amount "$FUND_AMOUNT"

FUNDED=$(stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source donor \
  --network "$NETWORK" \
  -- funded_total)
echo "  Funded total: $FUNDED"

# 5. Approver 1 approves milestone 0
echo ""
echo "[5/7] Approver 1 approves milestone 0..."
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source approver1 \
  --network "$NETWORK" \
  -- approve_milestone \
  --approver "$APPROVER1" \
  --index 0

APPROVALS=$(stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source approver1 \
  --network "$NETWORK" \
  -- approvals \
  --index 0)
echo "  Approvals so far: $APPROVALS / 2 needed"

# 6. Approver 2 approves milestone 0 → threshold reached → auto-release
echo ""
echo "[6/7] Approver 2 approves milestone 0 (threshold reached → auto-release)..."
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source approver2 \
  --network "$NETWORK" \
  -- approve_milestone \
  --approver "$APPROVER2" \
  --index 0

# 7. Read state
echo ""
echo "[7/7] Reading final state..."
STATE=$(stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source deployer \
  --network "$NETWORK" \
  -- state)
RELEASED=$(stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source deployer \
  --network "$NETWORK" \
  -- released_total)

echo ""
echo "=========================================="
echo " Demo complete!"
echo " Contract ID:     $CONTRACT_ID"
echo " State:           $STATE"
echo " Released total:  $RELEASED stroops"
echo "=========================================="
