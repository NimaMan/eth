#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"
MAINNET_RPC_URL="${MAINNET_RPC_URL:-http://127.0.0.1:8545}"

for name in VAULT_ADDRESS OWNER TREASURY UNIVERSAL_ROUTER PERMIT2 HOOKS_ALLOWED; do
  if [[ -z "${!name:-}" ]]; then
    echo "$name is required" >&2
    exit 1
  fi
done

mkdir -p "$RUN_DIR"

ACTUAL_OWNER="$(cast call "$VAULT_ADDRESS" "owner()(address)" --rpc-url "$MAINNET_RPC_URL")"
ACTUAL_TREASURY="$(cast call "$VAULT_ADDRESS" "treasury()(address)" --rpc-url "$MAINNET_RPC_URL")"
ACTUAL_ROUTER="$(cast call "$VAULT_ADDRESS" "universalRouter()(address)" --rpc-url "$MAINNET_RPC_URL")"
ACTUAL_PERMIT2="$(cast call "$VAULT_ADDRESS" "permit2()(address)" --rpc-url "$MAINNET_RPC_URL")"
ACTUAL_HOOKS_ALLOWED="$(cast call "$VAULT_ADDRESS" "hooksAllowed()(bool)" --rpc-url "$MAINNET_RPC_URL")"

jq -n \
  --arg vault_address "$VAULT_ADDRESS" \
  --arg expected_owner "$OWNER" \
  --arg actual_owner "$ACTUAL_OWNER" \
  --arg expected_treasury "$TREASURY" \
  --arg actual_treasury "$ACTUAL_TREASURY" \
  --arg expected_universal_router "$UNIVERSAL_ROUTER" \
  --arg actual_universal_router "$ACTUAL_ROUTER" \
  --arg expected_permit2 "$PERMIT2" \
  --arg actual_permit2 "$ACTUAL_PERMIT2" \
  --arg expected_hooks_allowed "$HOOKS_ALLOWED" \
  --arg actual_hooks_allowed "$ACTUAL_HOOKS_ALLOWED" \
  '{
    schema: "post_deploy_smoke_v1",
    vault_address: $vault_address,
    owner: {expected: $expected_owner, actual: $actual_owner},
    treasury: {expected: $expected_treasury, actual: $actual_treasury},
    universal_router: {expected: $expected_universal_router, actual: $actual_universal_router},
    permit2: {expected: $expected_permit2, actual: $actual_permit2},
    hooks_allowed: {expected: ($expected_hooks_allowed == "true"), actual: ($actual_hooks_allowed == "true")},
    passed: (
      ($expected_owner | ascii_downcase) == ($actual_owner | ascii_downcase) and
      ($expected_treasury | ascii_downcase) == ($actual_treasury | ascii_downcase) and
      ($expected_universal_router | ascii_downcase) == ($actual_universal_router | ascii_downcase) and
      ($expected_permit2 | ascii_downcase) == ($actual_permit2 | ascii_downcase) and
      (($expected_hooks_allowed == "true") == ($actual_hooks_allowed == "true"))
    )
  }' > "$RUN_DIR/post-deploy-smoke.json"

jq -e '.passed == true' "$RUN_DIR/post-deploy-smoke.json" >/dev/null

echo "run_dir=$RUN_DIR"
echo "post_deploy_smoke=$RUN_DIR/post-deploy-smoke.json"
