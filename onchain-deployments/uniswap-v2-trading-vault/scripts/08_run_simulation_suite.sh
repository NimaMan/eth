#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
ETH_ROOT="$(cd -- "${DEPLOY_DIR}/../.." && pwd)"
RUN_ID="${RUN_ID:-$(date -u +%Y%m%d-%H%M%SZ)}"
REPORT_DIR="${REPORT_DIR:-${DEPLOY_DIR}/simulations/reports/${RUN_ID}}"

USDC="0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
OTTO="0x12512a46d971BD035D9466246bc46DAf389E281c"
OTTO_AMOUNT="100000000000000000000"

mkdir -p "${REPORT_DIR}"

require_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "missing required tool: ${tool}" >&2
    exit 1
  fi
}

run_vault_case() {
  local name="$1"
  shift
  (
    cd "${ETH_ROOT}"
    cargo run -q -p tx_simulator --example uniswap_v2_trading_vault_deployed -- "$@"
  ) > "${REPORT_DIR}/${name}.json"
}

run_direct_case() {
  local name="$1"
  shift
  (
    cd "${ETH_ROOT}"
    cargo run -q -p tx_simulator --example uniswap_v2_direct_router -- "$@"
  ) > "${REPORT_DIR}/${name}.json"
}

assert_jq() {
  local file="$1"
  local expr="$2"
  local label="$3"
  if ! jq -e "${expr}" "${file}" >/dev/null; then
    echo "simulation assertion failed: ${label}" >&2
    echo "file: ${file}" >&2
    exit 1
  fi
}

require_tool cargo
require_tool jq

run_vault_case usdc-buy-then-sell \
  --scenario buy-then-sell \
  --token "${USDC}" \
  --symbol USDC \
  --buy-eth-wei 10000000000000000 \
  --min-tokens-out-raw 1 \
  --min-eth-out-wei 1

SIM_BLOCK="$(jq -r '.block' "${REPORT_DIR}/usdc-buy-then-sell.json")"

run_direct_case direct-usdc-buy-approve-then-sell \
  --block "${SIM_BLOCK}" \
  --scenario buy-approve-then-sell \
  --token "${USDC}" \
  --symbol USDC \
  --buy-eth-wei 10000000000000000 \
  --min-tokens-out-raw 1 \
  --min-eth-out-wei 1

run_vault_case otto-wallet-transfer-then-sell \
  --block "${SIM_BLOCK}" \
  --scenario wallet-transfer-then-sell \
  --token "${OTTO}" \
  --symbol OTTO \
  --transfer-amount-raw "${OTTO_AMOUNT}" \
  --min-eth-out-wei 1

run_direct_case direct-otto-wallet-approve-then-sell \
  --block "${SIM_BLOCK}" \
  --scenario wallet-approve-then-sell \
  --token "${OTTO}" \
  --symbol OTTO \
  --sell-amount-raw "${OTTO_AMOUNT}" \
  --min-eth-out-wei 1

run_vault_case otto-current-state-sell-negative \
  --block "${SIM_BLOCK}" \
  --scenario current-state-sell \
  --token "${OTTO}" \
  --symbol OTTO \
  --sell-amount-raw "${OTTO_AMOUNT}" \
  --min-eth-out-wei 1

assert_jq "${REPORT_DIR}/usdc-buy-then-sell.json" \
  '.buy.success == true and .vault_sell.success == true and .buy.gas_used <= 220000 and .vault_sell.gas_used <= 230000' \
  "USDC buy and emergency sell must succeed within gas ceiling"

assert_jq "${REPORT_DIR}/direct-usdc-buy-approve-then-sell.json" \
  '.direct_buy.success == true and .approve.success == true and .direct_sell.success == true and .direct_buy.gas_used <= 180000 and .approve.gas_used <= 80000 and .direct_sell.gas_used <= 180000' \
  "direct USDC buy, approve, and sell baseline must succeed within gas ceiling"

assert_jq "${REPORT_DIR}/otto-wallet-transfer-then-sell.json" \
  '.transfer.success == true and .vault_sell.success == true and .transfer.gas_used <= 130000 and .vault_sell.gas_used <= 260000' \
  "OTTO transfer and emergency sell must succeed within gas ceiling"

assert_jq "${REPORT_DIR}/direct-otto-wallet-approve-then-sell.json" \
  '.approve.success == true and .direct_sell.success == true and .approve.gas_used <= 80000 and .direct_sell.gas_used <= 220000' \
  "direct OTTO approve and sell baseline must succeed within gas ceiling"

assert_jq "${REPORT_DIR}/otto-current-state-sell-negative.json" \
  '.vault_sell.success == false and (.vault_sell.revert_reason | type == "string")' \
  "current-state sell without a vault position must fail in simulation"

jq -n \
  --arg schema "uniswap_v2_trading_vault_simulation_suite_v1" \
  --arg run_id "${RUN_ID}" \
  --arg report_dir "${REPORT_DIR}" \
  --slurpfile usdc_vault "${REPORT_DIR}/usdc-buy-then-sell.json" \
  --slurpfile usdc_direct "${REPORT_DIR}/direct-usdc-buy-approve-then-sell.json" \
  --slurpfile otto_vault "${REPORT_DIR}/otto-wallet-transfer-then-sell.json" \
  --slurpfile otto_direct "${REPORT_DIR}/direct-otto-wallet-approve-then-sell.json" \
  --slurpfile negative "${REPORT_DIR}/otto-current-state-sell-negative.json" \
  '{
    schema: $schema,
    run_id: $run_id,
    report_dir: $report_dir,
    passed: true,
    cases: {
      "usdc-buy-then-sell": {
        block: $usdc_vault[0].block,
        vault_buy_gas_used: $usdc_vault[0].buy.gas_used,
        vault_sell_gas_used: $usdc_vault[0].vault_sell.gas_used,
        vault_total_gas_used: ($usdc_vault[0].buy.gas_used + $usdc_vault[0].vault_sell.gas_used),
        direct_buy_gas_used: $usdc_direct[0].direct_buy.gas_used,
        direct_approve_gas_used: $usdc_direct[0].approve.gas_used,
        direct_sell_gas_used: $usdc_direct[0].direct_sell.gas_used,
        direct_total_gas_used: ($usdc_direct[0].direct_buy.gas_used + $usdc_direct[0].approve.gas_used + $usdc_direct[0].direct_sell.gas_used),
        vault_minus_direct_total_gas: (($usdc_vault[0].buy.gas_used + $usdc_vault[0].vault_sell.gas_used) - ($usdc_direct[0].direct_buy.gas_used + $usdc_direct[0].approve.gas_used + $usdc_direct[0].direct_sell.gas_used))
      },
      "otto-wallet-transfer-then-sell": {
        block: $otto_vault[0].block,
        vault_transfer_gas_used: $otto_vault[0].transfer.gas_used,
        vault_sell_gas_used: $otto_vault[0].vault_sell.gas_used,
        vault_total_gas_used: ($otto_vault[0].transfer.gas_used + $otto_vault[0].vault_sell.gas_used),
        direct_approve_gas_used: $otto_direct[0].approve.gas_used,
        direct_sell_gas_used: $otto_direct[0].direct_sell.gas_used,
        direct_total_gas_used: ($otto_direct[0].approve.gas_used + $otto_direct[0].direct_sell.gas_used),
        vault_minus_direct_total_gas: (($otto_vault[0].transfer.gas_used + $otto_vault[0].vault_sell.gas_used) - ($otto_direct[0].approve.gas_used + $otto_direct[0].direct_sell.gas_used))
      },
      "otto-current-state-sell-negative": {
        block: $negative[0].block,
        sell_success: $negative[0].vault_sell.success,
        revert_reason: $negative[0].vault_sell.revert_reason
      }
    }
  }' > "${REPORT_DIR}/summary.json"

echo "V2 deployed-vault simulation suite passed: ${REPORT_DIR}"
