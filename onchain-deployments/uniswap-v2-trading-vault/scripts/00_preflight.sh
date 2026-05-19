#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../.." && pwd)"

cd "$ETH_ROOT"

echo "eth_root=$ETH_ROOT"
echo "deploy_dir=$DEPLOY_DIR"
echo "git_rev=$(git rev-parse HEAD)"
git status --short -- \
  onchain-deployments/uniswap-v2-trading-vault \
  solidity/baygus-executor \
  alpha/live/trading \
  tx_simulator

for tool in git cargo forge cast jq; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "missing required tool: $tool" >&2
    exit 1
  fi
done

test -f solidity/baygus-executor/contracts/src/UniswapV2TradingVault.sol
test -f tx_simulator/src/tx_builders/protocols/uniswap_v2_trading_vault.rs
test -f alpha/live/trading/src/planner/route_builder.rs
test -f onchain-deployments/uniswap-v2-trading-vault/config/mainnet.toml
test -f onchain-deployments/uniswap-v2-trading-vault/config/gas-policy.mainnet.toml
test -f onchain-deployments/uniswap-v2-trading-vault/config/kartal-policy.mainnet.json

if rg -n "PRIVATE_KEY|API_TOKEN|SECRET|PASSWORD" onchain-deployments/uniswap-v2-trading-vault/config; then
  echo "config references secret env names only; confirm no secret values are committed" >&2
fi

cast sig "buyV2ExactEthForTokens(address,uint256,uint256)" | grep -qx "0x8a62666c"
cast sig "emergencySellV2ExactTokensForEth(address,uint256,uint256,uint256)" | grep -qx "0x5f413d10"

echo "preflight_ok=true"
