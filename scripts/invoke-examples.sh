#!/usr/bin/env bash
# scripts/invoke-examples.sh — initialize an example escrow on testnet.
#
# Usage:
#   ./scripts/invoke-examples.sh --contract <CONTRACT_ID> [--network <name>] [--source <identity>]
#
# This script calls `init` with a sample 3-approver, 2-of-3 threshold escrow
# with two milestones. Edit the variables below to match your actual deployment.
#
# Requirements:
#   - stellar CLI v25.2.0+ configured with funded testnet identities.
#   - The contract must already be deployed (see deploy-testnet.sh).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

SOURCE="deployer"
NETWORK="testnet"
CONTRACT_ID=""

# Parse flags.
while [[ $# -gt 0 ]]; do
  case "$1" in
    --contract) CONTRACT_ID="$2"; shift 2 ;;
    --source) SOURCE="$2"; shift 2 ;;
    --network) NETWORK="$2"; shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 1 ;;
  esac
done

if [[ -z "$CONTRACT_ID" ]]; then
  echo "ERROR: --contract <CONTRACT_ID> is required." >&2
  exit 1
fi

if ! command -v stellar &>/dev/null; then
  echo "ERROR: stellar CLI not found." >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Example escrow parameters — edit these for your actual use.
# ---------------------------------------------------------------------------

# Resolve identities to public keys (stellar CLI identity names or raw G-keys).
APPROVER1=$(stellar keys address approver1 --network "$NETWORK" 2>/dev/null || echo "GAPPROVER1REPLACE")
APPROVER2=$(stellar keys address approver2 --network "$NETWORK" 2>/dev/null || echo "GAPPROVER2REPLACE")
APPROVER3=$(stellar keys address approver3 --network "$NETWORK" 2>/dev/null || echo "GAPPROVER3REPLACE")
BENEFICIARY=$(stellar keys address beneficiary --network "$NETWORK" 2>/dev/null || echo "GBENEFICIARYREPLAC")

# USDC on testnet — replace with the actual SAC address for your token.
USDC_TOKEN="GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"

# Milestone amounts in stroops (7-decimal USDC).
# 100 USDC = 1_000_000_000 stroops; 200 USDC = 2_000_000_000 stroops.
MILESTONE_1=1000000000   # 100 USDC
MILESTONE_2=2000000000   # 200 USDC

# Expiry: current ledger + ~30 days (assuming ~5s per ledger, 30 days ≈ 518400 ledgers).
CURRENT_LEDGER=$(stellar ledger --network "$NETWORK" | jq -r '.sequence' 2>/dev/null || echo "0")
EXPIRY_LEDGER=$(( CURRENT_LEDGER + 518400 ))

echo "==> Initialising escrow on $NETWORK..."
echo "    Contract:    $CONTRACT_ID"
echo "    Beneficiary: $BENEFICIARY"
echo "    Approvers:   $APPROVER1, $APPROVER2, $APPROVER3"
echo "    Threshold:   2 of 3"
echo "    Milestones:  $MILESTONE_1 stroops, $MILESTONE_2 stroops"
echo "    Expiry:      ledger $EXPIRY_LEDGER"

stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE" \
  --network "$NETWORK" \
  -- init \
  --approvers "[$APPROVER1,$APPROVER2,$APPROVER3]" \
  --threshold 2 \
  --beneficiary "$BENEFICIARY" \
  --token "$USDC_TOKEN" \
  --milestones "[$MILESTONE_1,$MILESTONE_2]" \
  --expiry_ledger "$EXPIRY_LEDGER"

echo ""
echo "==> Escrow initialised!"
echo "    Fund it:    stellar contract invoke --id $CONTRACT_ID ... -- fund --donor <DONOR> --amount <AMOUNT>"
echo "    Approve:    stellar contract invoke --id $CONTRACT_ID ... -- approve_milestone --approver <APPROVER> --index 0"
echo "    Release:    stellar contract invoke --id $CONTRACT_ID ... -- release_milestone --index 0"
echo "    State:      stellar contract invoke --id $CONTRACT_ID ... -- state"
