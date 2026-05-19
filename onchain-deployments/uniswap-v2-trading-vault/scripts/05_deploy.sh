#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"
CONTRACT_ROOT="$ETH_ROOT/solidity/baygus-executor/contracts"
PRIVATE_KEY_ENV="${PRIVATE_KEY_ENV:-ETH_VAULT_DEPLOYER_PRIVATE_KEY}"

if [[ "${CONFIRM_DEPLOY:-}" != "1" ]]; then
  echo "Refusing to deploy. Set CONFIRM_DEPLOY=1 after completing audit and dry-run gates." >&2
  exit 1
fi

for name in MAINNET_RPC_URL OWNER TREASURY WETH UNISWAP_V2_ROUTER; do
  if [[ -z "${!name:-}" ]]; then
    echo "$name is required" >&2
    exit 1
  fi
done

if [[ -z "${!PRIVATE_KEY_ENV:-}" ]]; then
  echo "$PRIVATE_KEY_ENV is required and must contain the deployer private key" >&2
  exit 1
fi

mkdir -p "$RUN_DIR"
cd "$CONTRACT_ROOT"

jq -n \
  --arg owner "$OWNER" \
  --arg treasury "$TREASURY" \
  --arg weth "$WETH" \
  --arg uniswap_v2_router "$UNISWAP_V2_ROUTER" \
  --arg signer_env "$PRIVATE_KEY_ENV" \
  '{
    schema: "constructor_args_v1",
    owner: $owner,
    treasury: $treasury,
    weth: $weth,
    uniswap_v2_router: $uniswap_v2_router,
    signer_env: $signer_env
  }' > "$RUN_DIR/constructor-args.json"

set +x
forge create \
  src/UniswapV2TradingVault.sol:UniswapV2TradingVault \
  --rpc-url "$MAINNET_RPC_URL" \
  --private-key "${!PRIVATE_KEY_ENV}" \
  --constructor-args "$OWNER" "$TREASURY" "$WETH" "$UNISWAP_V2_ROUTER" \
  --json | tee "$RUN_DIR/deploy-output.json"

echo "run_dir=$RUN_DIR"
