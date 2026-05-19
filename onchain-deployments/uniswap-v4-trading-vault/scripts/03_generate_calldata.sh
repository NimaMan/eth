#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"
CONTRACT_ROOT="$ETH_ROOT/solidity/baygus-executor/contracts"
ARTIFACT="$CONTRACT_ROOT/../out/UniswapV4TradingVault.sol/UniswapV4TradingVault.json"
CONFIG="$DEPLOY_DIR/config/mainnet.candidate.toml"
FIXTURE="${FIXTURE:-$DEPLOY_DIR/simulations/route-fixtures/eth-usdc-500-no-hook.json}"
MAINNET_RPC_URL="${MAINNET_RPC_URL:-http://127.0.0.1:8545}"

config_value() {
  awk -F' = ' -v key="$1" '$1 == key {gsub(/"/, "", $2); print $2; exit}' "$CONFIG"
}

OWNER="${OWNER:-$(config_value owner)}"
TREASURY="${TREASURY:-$(config_value treasury)}"
UNIVERSAL_ROUTER="${UNIVERSAL_ROUTER:-$(config_value universal_router)}"
PERMIT2="${PERMIT2:-$(config_value permit2)}"
HOOKS_ALLOWED="${HOOKS_ALLOWED:-$(config_value hooks_allowed)}"
DEPLOYER="${DEPLOYER:-$(config_value signer)}"

mkdir -p "$RUN_DIR"
cd "$ETH_ROOT"
test -f "$ARTIFACT"
test -f "$FIXTURE"

BYTECODE="$(jq -r '.bytecode.object' "$ARTIFACT")"
CONSTRUCTOR_ARGS="$(cast abi-encode \
  "constructor(address,address,address,address,bool)" \
  "$OWNER" \
  "$TREASURY" \
  "$UNIVERSAL_ROUTER" \
  "$PERMIT2" \
  "$HOOKS_ALLOWED")"
INIT_CODE="${BYTECODE}${CONSTRUCTOR_ARGS#0x}"
DEPLOY_ESTIMATE="$(cast estimate \
  --rpc-url "$MAINNET_RPC_URL" \
  --create "$BYTECODE" \
  "constructor(address,address,address,address,bool)" \
  "$OWNER" "$TREASURY" "$UNIVERSAL_ROUTER" "$PERMIT2" "$HOOKS_ALLOWED")"
DEPLOYER_NONCE="$(cast nonce "$DEPLOYER" --rpc-url "$MAINNET_RPC_URL")"
PREDICTED_ADDRESS="$(cast compute-address --nonce "$DEPLOYER_NONCE" "$DEPLOYER" | awk -F': ' '/Computed Address:/ {print $2; exit}')"

CURRENCY0="$(jq -r '.pool_key.currency0' "$FIXTURE")"
CURRENCY1="$(jq -r '.pool_key.currency1' "$FIXTURE")"
FEE="$(jq -r '.pool_key.fee' "$FIXTURE")"
TICK_SPACING="$(jq -r '.pool_key.tick_spacing' "$FIXTURE")"
HOOKS="$(jq -r '.pool_key.hooks' "$FIXTURE")"
TOKEN_OUT="$(jq -r '.token_out' "$FIXTURE")"
DEADLINE="$(jq -r '.deadline_raw' "$FIXTURE")"
HOOK_DATA="$(jq -r '.hook_data_hex // "0x"' "$FIXTURE")"
MIN_TOKENS_OUT="${MIN_TOKENS_OUT:-$(jq -r '.min_amount_out_raw' "$FIXTURE")}"
TOKEN_AMOUNT_IN="${TOKEN_AMOUNT_IN:-1000000}"
MIN_ETH_OUT="${MIN_ETH_OUT:-$(jq -r '.sell_min_eth_out_wei' "$FIXTURE")}"
POOL_KEY="($CURRENCY0,$CURRENCY1,$FEE,$TICK_SPACING,$HOOKS)"

BUY_DATA="$(cast calldata \
  "buyV4ExactEthForTokens((address,address,uint24,int24,address),address,uint128,uint256,bytes)" \
  "$POOL_KEY" \
  "$TOKEN_OUT" \
  "$MIN_TOKENS_OUT" \
  "$DEADLINE" \
  "$HOOK_DATA")"

SELL_DATA="$(cast calldata \
  "emergencySellV4ExactTokensForEth((address,address,uint24,int24,address),address,uint128,uint128,uint256,bytes)" \
  "$POOL_KEY" \
  "$TOKEN_OUT" \
  "$TOKEN_AMOUNT_IN" \
  "$MIN_ETH_OUT" \
  "$DEADLINE" \
  "$HOOK_DATA")"

jq -n \
  --arg owner "$OWNER" \
  --arg treasury "$TREASURY" \
  --arg universal_router "$UNIVERSAL_ROUTER" \
  --arg permit2 "$PERMIT2" \
  --arg hooks_allowed "$HOOKS_ALLOWED" \
  --arg deployer "$DEPLOYER" \
  '{
    schema: "constructor_args_v1",
    owner: $owner,
    treasury: $treasury,
    universal_router: $universal_router,
    permit2: $permit2,
    hooks_allowed: ($hooks_allowed == "true"),
    deployer: $deployer
  }' > "$RUN_DIR/constructor-args.json"

jq -n \
  --arg fixture "$FIXTURE" \
  --arg deployer "$DEPLOYER" \
  --arg deployer_nonce "$DEPLOYER_NONCE" \
  --arg predicted_address "$PREDICTED_ADDRESS" \
  --arg constructor_args "$CONSTRUCTOR_ARGS" \
  --arg init_code "$INIT_CODE" \
  --arg init_code_hash "$(cast keccak "$INIT_CODE")" \
  --arg deploy_estimate_gas "$DEPLOY_ESTIMATE" \
  --arg pool_key "$POOL_KEY" \
  --arg token_out "$TOKEN_OUT" \
  --arg min_tokens_out "$MIN_TOKENS_OUT" \
  --arg token_amount_in "$TOKEN_AMOUNT_IN" \
  --arg min_eth_out "$MIN_ETH_OUT" \
  --arg deadline "$DEADLINE" \
  --arg hook_data "$HOOK_DATA" \
  --arg buy_data "$BUY_DATA" \
  --arg sell_data "$SELL_DATA" \
  '{
    schema: "uniswap_v4_trading_vault_calldata_v1",
    fixture: $fixture,
    deployment: {
      deployer: $deployer,
      deployer_nonce: $deployer_nonce,
      predicted_address: $predicted_address,
      constructor_signature: "constructor(address,address,address,address,bool)",
      constructor_args: $constructor_args,
      init_code_hash: $init_code_hash,
      estimate_gas: $deploy_estimate_gas,
      init_code: $init_code
    },
    route: {
      pool_key: $pool_key,
      token_out: $token_out,
      min_tokens_out: $min_tokens_out,
      token_amount_in: $token_amount_in,
      min_eth_out: $min_eth_out,
      deadline: $deadline,
      hook_data: $hook_data
    },
    buy: {
      method: "buyV4ExactEthForTokens((address,address,uint24,int24,address),address,uint128,uint256,bytes)",
      selector: "0x1c2ecb7f",
      data: $buy_data
    },
    emergency_sell: {
      method: "emergencySellV4ExactTokensForEth((address,address,uint24,int24,address),address,uint128,uint128,uint256,bytes)",
      selector: "0x3a832d08",
      data: $sell_data
    }
  }' > "$RUN_DIR/calldata.json"

jq -n \
  --arg route_fixture "$FIXTURE" \
  --argjson fork_block "${FORK_BLOCK:-25131251}" \
  --arg owner "$OWNER" \
  --arg treasury "$TREASURY" \
  --arg predicted_address "$PREDICTED_ADDRESS" \
  '{
    schema: "uniswap_v4_trading_vault_run_inputs_v1",
    status: "candidate",
    route_fixture: $route_fixture,
    fork_block: $fork_block,
    owner: $owner,
    treasury: $treasury,
    predicted_address: $predicted_address
  }' > "$RUN_DIR/inputs.json"

echo "run_dir=$RUN_DIR"
echo "calldata=$RUN_DIR/calldata.json"
