#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"

for name in MAINNET_RPC_URL VAULT_ADDRESS OWNER TREASURY WETH UNISWAP_V2_ROUTER; do
  if [[ -z "${!name:-}" ]]; then
    echo "$name is required" >&2
    exit 1
  fi
done

mkdir -p "$RUN_DIR"

ACTUAL_OWNER="$(cast call "$VAULT_ADDRESS" "owner()(address)" --rpc-url "$MAINNET_RPC_URL")"
ACTUAL_TREASURY="$(cast call "$VAULT_ADDRESS" "treasury()(address)" --rpc-url "$MAINNET_RPC_URL")"
ACTUAL_WETH="$(cast call "$VAULT_ADDRESS" "weth()(address)" --rpc-url "$MAINNET_RPC_URL")"
ACTUAL_ROUTER="$(cast call "$VAULT_ADDRESS" "uniswapV2Router()(address)" --rpc-url "$MAINNET_RPC_URL")"

jq -n \
  --arg vault_address "$VAULT_ADDRESS" \
  --arg expected_owner "$OWNER" \
  --arg actual_owner "$ACTUAL_OWNER" \
  --arg expected_treasury "$TREASURY" \
  --arg actual_treasury "$ACTUAL_TREASURY" \
  --arg expected_weth "$WETH" \
  --arg actual_weth "$ACTUAL_WETH" \
  --arg expected_uniswap_v2_router "$UNISWAP_V2_ROUTER" \
  --arg actual_uniswap_v2_router "$ACTUAL_ROUTER" \
  '{
    schema: "post_deploy_smoke_v1",
    vault_address: $vault_address,
    owner: {expected: $expected_owner, actual: $actual_owner},
    treasury: {expected: $expected_treasury, actual: $actual_treasury},
    weth: {expected: $expected_weth, actual: $actual_weth},
    uniswap_v2_router: {expected: $expected_uniswap_v2_router, actual: $actual_uniswap_v2_router},
    passed: (
      ($expected_owner | ascii_downcase) == ($actual_owner | ascii_downcase) and
      ($expected_treasury | ascii_downcase) == ($actual_treasury | ascii_downcase) and
      ($expected_weth | ascii_downcase) == ($actual_weth | ascii_downcase) and
      ($expected_uniswap_v2_router | ascii_downcase) == ($actual_uniswap_v2_router | ascii_downcase)
    )
  }' > "$RUN_DIR/post-deploy-smoke.json"

jq -e '.passed == true' "$RUN_DIR/post-deploy-smoke.json" >/dev/null

echo "run_dir=$RUN_DIR"
echo "post_deploy_smoke=$RUN_DIR/post-deploy-smoke.json"
