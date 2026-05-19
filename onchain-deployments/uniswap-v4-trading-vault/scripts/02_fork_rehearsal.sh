#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"
CONTRACT_ROOT="$ETH_ROOT/solidity/baygus-executor/contracts"
MAINNET_RPC_URL="${MAINNET_RPC_URL:-http://127.0.0.1:8545}"
FORK_BLOCK="${FORK_BLOCK:-25131251}"

mkdir -p "$RUN_DIR"
cd "$CONTRACT_ROOT"

forge test \
  --match-contract UniswapV4TradingVaultForkGasTest \
  --fork-url "$MAINNET_RPC_URL" \
  --fork-block-number "$FORK_BLOCK" \
  --gas-report -vv | tee "$RUN_DIR/fork-rehearsal.txt"

forge snapshot \
  --match-contract UniswapV4TradingVaultForkGasTest \
  --fork-url "$MAINNET_RPC_URL" \
  --fork-block-number "$FORK_BLOCK" \
  --snap "$RUN_DIR/gas-snapshot.txt"

cd "$ETH_ROOT"
cargo run -q -p tx_simulator \
  --example uniswap_v4_trading_vault_candidate_rehearsal \
  -- --block "$FORK_BLOCK" \
  | tee "$RUN_DIR/fork-rehearsal-tx-simulator.json" >/dev/null

jq -n \
  --argjson fork_block "$FORK_BLOCK" \
  --arg gas_snapshot "$RUN_DIR/gas-snapshot.txt" \
  --slurpfile simulator "$RUN_DIR/fork-rehearsal-tx-simulator.json" \
  '{
    schema: "fork_rehearsal_v1",
    fork_block: $fork_block,
    gas_snapshot: $gas_snapshot,
    tx_simulator: $simulator[0],
    status: "completed"
  }' > "$RUN_DIR/fork-rehearsal.json"

echo "run_dir=$RUN_DIR"
