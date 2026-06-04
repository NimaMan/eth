#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../../.." && pwd)"

cd "$ETH_ROOT"

echo "eth_root=$ETH_ROOT"
echo "deploy_dir=$DEPLOY_DIR"
echo "git_rev=$(git rev-parse HEAD)"
git status --short -- \
  deploy/onchain/uniswap-v4-trading-vault \
  solidity/baygus-executor \
  tx_simulator

for tool in git cargo forge cast jq rg curl; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "missing required tool: $tool" >&2
    exit 1
  fi
done

test -f solidity/baygus-executor/contracts/src/v4/UniswapV4TradingVault.sol
test -f solidity/baygus-executor/contracts/test/v4/UniswapV4TradingVault.t.sol
test -f solidity/baygus-executor/contracts/test/bench/v4/UniswapV4TradingVaultForkGas.t.sol
test -f tx_simulator/src/tx_builders/protocols/uniswap/v4/trading_vault/mod.rs
test -f deploy/onchain/uniswap-v4-trading-vault/config/mainnet.candidate.toml
test -f deploy/onchain/uniswap-v4-trading-vault/config/gas-policy.candidate.toml
test -f deploy/onchain/uniswap-v4-trading-vault/config/eth-tx-policy.candidate.json
test -f deploy/onchain/uniswap-v4-trading-vault/simulations/route-fixtures/eth-usdc-500-no-hook.json

jq -e '.schema == "uniswap_v4_trading_vault_eth_tx_policy_candidate_v1"' \
  deploy/onchain/uniswap-v4-trading-vault/config/eth-tx-policy.candidate.json >/dev/null
jq -e '.schema == "uniswap_v4_route_fixture_v1"' \
  deploy/onchain/uniswap-v4-trading-vault/simulations/route-fixtures/eth-usdc-500-no-hook.json >/dev/null

cast sig "buyV4ExactEthForTokens((address,address,uint24,int24,address),address,uint128,uint256,bytes)" \
  | grep -qx "0x1c2ecb7f"
cast sig "emergencySellV4ExactTokensForEth((address,address,uint24,int24,address),address,uint128,uint128,uint256,bytes)" \
  | grep -qx "0x3a832d08"

if rg -n "PRIVATE_KEY|API_TOKEN|SECRET|PASSWORD" deploy/onchain/uniswap-v4-trading-vault/config; then
  echo "config references secret env names only; confirm no secret values are committed" >&2
fi

echo "preflight_ok=true"
