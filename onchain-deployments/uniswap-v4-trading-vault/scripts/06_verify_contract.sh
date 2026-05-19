#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"
CONTRACT_ROOT="$ETH_ROOT/solidity/baygus-executor/contracts"

for name in VAULT_ADDRESS OWNER TREASURY UNIVERSAL_ROUTER PERMIT2 HOOKS_ALLOWED ETHERSCAN_API_KEY; do
  if [[ -z "${!name:-}" ]]; then
    echo "$name is required" >&2
    exit 1
  fi
done

mkdir -p "$RUN_DIR"
cd "$CONTRACT_ROOT"

CONSTRUCTOR_ARGS="$(cast abi-encode \
  "constructor(address,address,address,address,bool)" \
  "$OWNER" \
  "$TREASURY" \
  "$UNIVERSAL_ROUTER" \
  "$PERMIT2" \
  "$HOOKS_ALLOWED")"

forge verify-contract \
  "$VAULT_ADDRESS" \
  src/v4/UniswapV4TradingVault.sol:UniswapV4TradingVault \
  --chain-id 1 \
  --constructor-args "$CONSTRUCTOR_ARGS" \
  --etherscan-api-key "$ETHERSCAN_API_KEY" \
  --watch | tee "$RUN_DIR/verification-output.txt"

jq -n \
  --arg vault_address "$VAULT_ADDRESS" \
  --arg constructor_args "$CONSTRUCTOR_ARGS" \
  --arg output "$RUN_DIR/verification-output.txt" \
  '{
    schema: "verification_output_v1",
    vault_address: $vault_address,
    constructor_args: $constructor_args,
    output: $output
  }' > "$RUN_DIR/verification-output.json"

echo "run_dir=$RUN_DIR"
