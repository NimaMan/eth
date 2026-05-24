#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../../.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"
CALLDATA_JSON="${CALLDATA_JSON:-$RUN_DIR/calldata.json}"
KARTAL_URL="${KARTAL_URL:-http://127.0.0.1:5004}"
KARTAL_ENV="${KARTAL_ENV:-/home/nima/code/crypto/kartal/.env}"
MAINNET_RPC_URL="${MAINNET_RPC_URL:-http://127.0.0.1:8545}"
EXPECT="${EXPECT:-policy-rejected}"
FROM="${FROM:-0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27}"
VAULT_ADDRESS="${VAULT_ADDRESS:-}"
BUY_VALUE_WEI="${BUY_VALUE_WEI:-10000000000000000}"
MAX_FEE_PER_GAS_WEI="${MAX_FEE_PER_GAS_WEI:-10000000000}"
MAX_PRIORITY_FEE_PER_GAS_WEI="${MAX_PRIORITY_FEE_PER_GAS_WEI:-1000000000}"

mkdir -p "$RUN_DIR"
cd "$ETH_ROOT"

if [[ ! -f "$CALLDATA_JSON" ]]; then
  echo "missing calldata JSON: $CALLDATA_JSON; run 03_generate_calldata.sh first or set CALLDATA_JSON" >&2
  exit 1
fi

if [[ -z "$VAULT_ADDRESS" ]]; then
  VAULT_ADDRESS="$(jq -r '.deployment.predicted_address // empty' "$CALLDATA_JSON")"
fi
if [[ -z "$VAULT_ADDRESS" ]]; then
  echo "VAULT_ADDRESS is required, or calldata.json must include deployment.predicted_address" >&2
  exit 1
fi

TOKEN_ADDRESS="$(jq -r '.route.token_out' "$CALLDATA_JSON")"
POOL_ADDRESS="v4:$(jq -r '.route.pool_key' "$CALLDATA_JSON")"
SIMULATION_BLOCK="${SIMULATION_BLOCK:-$(cast block-number --rpc-url "$MAINNET_RPC_URL")}"
BUY_DATA="$(jq -r '.buy.data' "$CALLDATA_JSON")"
SELL_DATA="$(jq -r '.emergency_sell.data' "$CALLDATA_JSON")"

if [[ -z "${KARTAL_API_TOKEN:-}" && -f "$KARTAL_ENV" ]]; then
  KARTAL_API_TOKEN="$(awk -F= '/^KARTAL_API_TOKEN=/{print $2; exit}' "$KARTAL_ENV")"
fi
if [[ -z "${ETH_TX_EXECUTOR_API_TOKEN:-}" && -z "${KARTAL_API_TOKEN:-}" ]]; then
  echo "KARTAL_API_TOKEN or ETH_TX_EXECUTOR_API_TOKEN is required" >&2
  exit 1
fi
TOKEN="${ETH_TX_EXECUTOR_API_TOKEN:-$KARTAL_API_TOKEN}"

make_request() {
  local kind="$1"
  local data="$2"
  local value="$3"
  local gas_limit="$4"
  local output="$5"

  jq -n \
    --arg attempt_id "v4-vault-$kind-$(date -u +%Y%m%d-%H%M%SZ)" \
    --arg from "$FROM" \
    --arg to "$VAULT_ADDRESS" \
    --arg value "$value" \
    --arg data "$data" \
    --arg gas_limit "$gas_limit" \
    --arg max_fee_per_gas "$MAX_FEE_PER_GAS_WEI" \
    --arg max_priority_fee_per_gas "$MAX_PRIORITY_FEE_PER_GAS_WEI" \
    --argjson simulation_block "$SIMULATION_BLOCK" \
    --arg token_address "$TOKEN_ADDRESS" \
    --arg pool_address "$POOL_ADDRESS" \
    --arg intent_kind "$kind" \
    '{
      attempt_id: $attempt_id,
      chain_id: 1,
      from: $from,
      to: $to,
      value: $value,
      data: $data,
      gas_limit: $gas_limit,
      max_fee_per_gas: $max_fee_per_gas,
      max_priority_fee_per_gas: $max_priority_fee_per_gas,
      nonce: null,
      bribe: null,
      simulation: {
        block_number: $simulation_block,
        block_hash: null,
        state_root: null,
        expected_output_token: $token_address,
        expected_output_amount: null,
        min_output_amount: "1",
        metadata: {source: "uniswap_v4_trading_vault_gate"}
      },
      metadata: {
        wire_protocol: "eth_direct_raw_v1",
        intent_kind: $intent_kind,
        strategy_name: "uniswap-v4-trading-vault-deploy-gate",
        strategy_run_id: "manual-v4-vault-gate",
        trade_id: $attempt_id,
        token_address: $token_address,
        pool_address: $pool_address,
        executor_boundary: "kartal_eth_tx_executor",
        tx_prep_version: 1
      }
    }' > "$output"
}

make_request "v4_vault_buy" "$BUY_DATA" "$BUY_VALUE_WEI" "350000" "$RUN_DIR/kartal-buy-request.json"
make_request "v4_vault_emergency_sell" "$SELL_DATA" "0" "450000" "$RUN_DIR/kartal-sell-request.json"

submit_request() {
  local label="$1"
  local request="$2"
  local body_file="$RUN_DIR/kartal-$label-response-body.json"
  local code_file="$RUN_DIR/kartal-$label-http-code.txt"

  curl -sS \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -o "$body_file" \
    -w "%{http_code}" \
    -X POST "$KARTAL_URL/eth/tx/direct-raw" \
    --data-binary "@$request" > "$code_file"
}

submit_request "buy" "$RUN_DIR/kartal-buy-request.json"
submit_request "sell" "$RUN_DIR/kartal-sell-request.json"

STATUS_BODY="$RUN_DIR/kartal-status.json"
curl -fsS \
  -H "Authorization: Bearer $TOKEN" \
  "$KARTAL_URL/eth/tx/status" > "$STATUS_BODY"

jq -n \
  --arg schema "uniswap_v4_trading_vault_kartal_dry_run_v1" \
  --arg kartal_url "$KARTAL_URL" \
  --arg expect "$EXPECT" \
  --arg vault_address "$VAULT_ADDRESS" \
  --arg simulation_block "$SIMULATION_BLOCK" \
  --slurpfile status "$STATUS_BODY" \
  --slurpfile buy_request "$RUN_DIR/kartal-buy-request.json" \
  --rawfile buy_response_raw "$RUN_DIR/kartal-buy-response-body.json" \
  --rawfile buy_http_code "$RUN_DIR/kartal-buy-http-code.txt" \
  --slurpfile sell_request "$RUN_DIR/kartal-sell-request.json" \
  --rawfile sell_response_raw "$RUN_DIR/kartal-sell-response-body.json" \
  --rawfile sell_http_code "$RUN_DIR/kartal-sell-http-code.txt" \
  '{
    schema: $schema,
    kartal_url: $kartal_url,
    expect: $expect,
    vault_address: $vault_address,
    simulation_block: ($simulation_block | tonumber),
    status: $status[0],
    buy: {
      http_code: ($buy_http_code | tonumber),
      request: $buy_request[0],
      response: ($buy_response_raw | fromjson? // {raw: $buy_response_raw})
    },
    sell: {
      http_code: ($sell_http_code | tonumber),
      request: $sell_request[0],
      response: ($sell_response_raw | fromjson? // {raw: $sell_response_raw})
    }
  }' > "$RUN_DIR/kartal-dry-run.json"

case "$EXPECT" in
  accepted)
    jq -e '.buy.http_code == 200 and .sell.http_code == 200' "$RUN_DIR/kartal-dry-run.json" >/dev/null
    ;;
  policy-rejected)
    jq -e '.buy.http_code >= 400 and .sell.http_code >= 400' "$RUN_DIR/kartal-dry-run.json" >/dev/null
    ;;
  *)
    echo "invalid EXPECT=$EXPECT; expected accepted or policy-rejected" >&2
    exit 1
    ;;
esac

echo "run_dir=$RUN_DIR"
echo "kartal_dry_run=$RUN_DIR/kartal-dry-run.json"
