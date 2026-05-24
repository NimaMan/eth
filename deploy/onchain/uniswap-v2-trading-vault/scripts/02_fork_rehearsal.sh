#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../../.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"
CONTRACT_ROOT="$ETH_ROOT/solidity/baygus-executor/contracts"
FORK_BLOCK="${FORK_BLOCK:-25129486}"

if [[ -z "${MAINNET_RPC_URL:-}" ]]; then
  echo "MAINNET_RPC_URL is required for fork rehearsal" >&2
  exit 1
fi

mkdir -p "$RUN_DIR"
cd "$CONTRACT_ROOT"

forge test \
  --match-contract UniswapV2TradingVaultForkGasTest \
  --fork-url "$MAINNET_RPC_URL" \
  --fork-block-number "$FORK_BLOCK" \
  --gas-report | tee "$RUN_DIR/fork-rehearsal.txt"

forge snapshot \
  --match-contract UniswapV2TradingVaultForkGasTest \
  --fork-url "$MAINNET_RPC_URL" \
  --fork-block-number "$FORK_BLOCK" \
  --snap "$RUN_DIR/gas-snapshot.txt"

jq -n \
  --argjson fork_block "$FORK_BLOCK" \
  --arg gas_snapshot "$RUN_DIR/gas-snapshot.txt" \
  '{
    schema: "fork_rehearsal_v1",
    fork_block: $fork_block,
    gas_snapshot: $gas_snapshot,
    status: "completed"
  }' > "$RUN_DIR/fork-rehearsal.json"

echo "run_dir=$RUN_DIR"
