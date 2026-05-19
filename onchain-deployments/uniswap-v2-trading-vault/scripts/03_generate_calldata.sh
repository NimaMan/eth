#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"

if [[ -z "${TOKEN:-}" ]]; then
  echo "TOKEN is required" >&2
  exit 1
fi

if [[ -z "${DEADLINE:-}" ]]; then
  echo "DEADLINE is required" >&2
  exit 1
fi

MIN_TOKENS_OUT="${MIN_TOKENS_OUT:-0}"
TOKEN_AMOUNT_IN="${TOKEN_AMOUNT_IN:-0}"
MIN_ETH_OUT="${MIN_ETH_OUT:-0}"

mkdir -p "$RUN_DIR"

BUY_DATA="$(cast calldata \
  "buyV2ExactEthForTokens(address,uint256,uint256)" \
  "$TOKEN" \
  "$MIN_TOKENS_OUT" \
  "$DEADLINE")"

SELL_DATA="$(cast calldata \
  "emergencySellV2ExactTokensForEth(address,uint256,uint256,uint256)" \
  "$TOKEN" \
  "$TOKEN_AMOUNT_IN" \
  "$MIN_ETH_OUT" \
  "$DEADLINE")"

jq -n \
  --arg token "$TOKEN" \
  --arg min_tokens_out "$MIN_TOKENS_OUT" \
  --arg token_amount_in "$TOKEN_AMOUNT_IN" \
  --arg min_eth_out "$MIN_ETH_OUT" \
  --arg deadline "$DEADLINE" \
  --arg buy_data "$BUY_DATA" \
  --arg sell_data "$SELL_DATA" \
  '{
    schema: "uniswap_v2_trading_vault_calldata_v1",
    token: $token,
    min_tokens_out: $min_tokens_out,
    token_amount_in: $token_amount_in,
    min_eth_out: $min_eth_out,
    deadline: $deadline,
    buy: {
      method: "buyV2ExactEthForTokens(address,uint256,uint256)",
      selector: "0x8a62666c",
      data: $buy_data
    },
    emergency_sell: {
      method: "emergencySellV2ExactTokensForEth(address,uint256,uint256,uint256)",
      selector: "0x5f413d10",
      data: $sell_data
    }
  }' > "$RUN_DIR/calldata.json"

echo "run_dir=$RUN_DIR"
echo "calldata=$RUN_DIR/calldata.json"
