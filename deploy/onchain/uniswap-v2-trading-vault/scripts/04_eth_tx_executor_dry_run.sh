#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../../.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"
REQUEST="${REQUEST:-$ETH_ROOT/alpha/live/trading/fixtures/eth_tx_calibration/reject_policy_request.json}"
ETH_TX_EXECUTOR_URL="${ETH_TX_EXECUTOR_URL:-http://127.0.0.1:5006}"
EXPECT="${EXPECT:-policy-rejected}"

mkdir -p "$RUN_DIR"
cd "$ETH_ROOT"

cargo run -p eth_alpha_engine --bin eth_alpha_tx_executor_calibrate -- \
  --request "$REQUEST" \
  --eth-tx-executor-url "$ETH_TX_EXECUTOR_URL" \
  --expect "$EXPECT" \
  --report-path "$RUN_DIR/eth-tx-executor-dry-run.json" \
  --print-json | tee "$RUN_DIR/eth-tx-executor-dry-run.txt"

echo "run_dir=$RUN_DIR"
