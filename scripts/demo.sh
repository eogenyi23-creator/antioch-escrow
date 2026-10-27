#!/usr/bin/env bash
# scripts/demo.sh — end-to-end demo: deploy, fund, approve twice, release.
#
# Usage:
#   ./scripts/demo.sh [--network <name>]
#
# What it does:
#   1. Builds the contract WASM.
#   2. Deploys it to testnet with constructor args (atomic deploy + init).
#   3. Funds the escrow from a demo donor.
#   4. Approver 1 approves milestone 0.
#   5. Approver 2 approves milestone 0 (threshold reached → auto-release).
#   6. Reads state and prints a summary.
#
# Token setup:
#   The contract requires a Stellar Asset Contract (SAC) token address.
#   SAC contract addresses start with "C" (they are contract IDs, not issuer
#   G-addresses). Set the USDC_TOKEN env var to a C-address SAC contract ID:
#
#     USDC_TOKEN=Cxxxx... ./scripts/demo.sh
#
#   To get the SAC contract ID for an existing asset on testnet:
#     stellar contract asset deploy --asset USDC:issuer --source deployer --network testnet
#
#   Or, to deploy your own test asset and get its SAC contract ID:
#     stellar contract asset deploy --asset TEST:Gxxxx --source deployer --network testnet
#
#   If USDC_TOKEN is not set, the script will deploy a new test asset and use its
#   SAC contract address automatically.
#
#   Donors must have a trustline to the asset and a balance. Set up with:
#     stellar trustline donor --asset USDC:issuer --network testnet
#     # then mint or transfer tokens to the donor address
#
# Requirements:
#   - stellar CLI v25.2.0+ with the following funded testnet identities:
#       deployer, approver1, approver2, approver3, donor, beneficiary
#     Create them with:
#       for id in deployer approver1 approver2 approver3 donor beneficiary; do
#         stellar keys generate --network testnet $id
#         stellar keys fund --network testnet $id
#       done

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

NETWORK="${NETWORK:-testnet}"

# Fund amount: 500 USDC in stroops (7 decimals).
FUND_AMOUNT=5000000000

# Milestone: 500 USDC in stroops.
MILESTONE_AMOUNT=5000000000

if ! command -v stellar &>/dev/null; then
  echo "ERROR: stellar CLI not found." >&2
  exit 1
fi

# Helper: resolve identity to public key.
addr() { stellar keys address "$1" --network "$NETWORK"; }

# ---------------------------------------------------------------------------
# Token setup: resolve or deploy a SAC token contract.
# ---------------------------------------------------------------------------

if [[ -n "${USDC_TOKEN:-}" ]]; then
  # User provided a C-address SAC contract ID.
  TOKEN_ADDR="$USDC_TOKEN"
  echo "Using provided SAC token contract: $TOKEN_ADDR"
else
  # Deploy a test asset and get its SAC contract address.
  echo "USDC_TOKEN not set — deploying a new test asset..."
  ISSUER=$(addr deployer)

  # Deploy a test asset SAC. The stellar CLI creates the asset and wraps it
  # in a SAC contract, returning the C-address contract ID.
  TOKEN_ADDR=$(stellar contract asset deploy \
    --asset "DEMO:${ISSUER}" \
    --source deployer \
    --network "$NETWORK")

  echo "  Deployed test asset SAC: $TOKEN_ADDR"

  # Ensure the donor has a trustline to the test asset.
  stellar trustline donor --asset "DEMO:${ISSUER}" --network "$NETWORK" || true

  # Mint tokens to the donor.
  DONOR_ADDR=$(addr donor)
  stellar contract invoke \
    --id "$TOKEN_ADDR" \
    --source deployer \
    --network "$NETWORK" \
    -- mint \
    --to "$DONOR_ADDR" \
    --amount "$FUND_AMOUNT"
fi

echo "=========================================="
echo " Antioch Escrow — end-to-end demo"
echo " Network: $NETWORK"
echo " Token:   $TOKEN_ADDR"
echo "=========================================="
echo ""

# 1. Build
echo "[1/7] Building contract..."
./scripts/build.sh

# 2. Deploy with constructor (atomic deploy + init)
echo ""
echo "[2/7] Deploying contract with constructor..."

APPROVER1=$(addr approver1)
APPROVER2=$(addr approver2)
APPROVER3=$(addr approver3)
BENEFICIARY=$(addr beneficiary)
DONOR=$(addr donor)

CURRENT_LEDGER=$(stellar ledger --network "$NETWORK" | jq -r '.sequence')
# Expiry: 10 minutes from now (~120 ledgers at ~5s each).
EXPIRY=$(( CURRENT_LEDGER + 120 ))

CONTRACT_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/escrow.wasm \
  --source deployer \
  --network "$NETWORK" \
  -- \
  __constructor \
  --approvers "[$APPROVER1,$APPROVER2,$APPROVER3]" \
  --threshold 2 \
  --beneficiary "$BENEFICIARY" \
  --token "$TOKEN_ADDR" \
  --milestones "[$MILESTONE_AMOUNT]" \
  --expiry_ledger "$EXPIRY")

echo "  Contract ID: $CONTRACT_ID"

# 3. Fund
echo ""
echo "[3/7] Funding escrow ($FUND_AMOUNT stroops from donor)..."
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

# 4. Approver 1 approves milestone 0
echo ""
echo "[4/7] Approver 1 approves milestone 0..."
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

# 5. Approver 2 approves milestone 0 → threshold reached → auto-release
echo ""
echo "[5/7] Approver 2 approves milestone 0 (threshold reached → auto-release)..."
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source approver2 \
  --network "$NETWORK" \
  -- approve_milestone \
  --approver "$APPROVER2" \
  --index 0

# 6. Read state
echo ""
echo "[6/7] Reading final state..."
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
