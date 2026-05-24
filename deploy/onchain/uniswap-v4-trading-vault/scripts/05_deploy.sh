#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../../.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"
CONTRACT_ROOT="$ETH_ROOT/solidity/baygus-executor/contracts"
PRIVATE_KEY_ENV="${PRIVATE_KEY_ENV:-ETH_VAULT_DEPLOYER_PRIVATE_KEY}"
SIGNER_BACKEND="${SIGNER_BACKEND:-keystore}"
KEYSTORE_PATH="${KEYSTORE_PATH:-${ETH_VAULT_DEPLOYER_KEYSTORE:-}}"
PASSWORD_FILE="${PASSWORD_FILE:-${ETH_VAULT_DEPLOYER_PASSWORD_FILE:-}}"

if [[ "${CONFIRM_DEPLOY:-}" != "1" ]]; then
  echo "Refusing to deploy. Set CONFIRM_DEPLOY=1 after completing audit and dry-run gates." >&2
  exit 1
fi

for name in MAINNET_RPC_URL OWNER TREASURY UNIVERSAL_ROUTER PERMIT2 HOOKS_ALLOWED; do
  if [[ -z "${!name:-}" ]]; then
    echo "$name is required" >&2
    exit 1
  fi
done

case "$SIGNER_BACKEND" in
  private_key_env)
    if [[ -z "${!PRIVATE_KEY_ENV:-}" ]]; then
      echo "$PRIVATE_KEY_ENV is required and must contain the deployer private key" >&2
      exit 1
    fi
    SIGNER_ARGS=(--private-key "${!PRIVATE_KEY_ENV}")
    SIGNER_RECORD="$(jq -n --arg backend "$SIGNER_BACKEND" --arg env "$PRIVATE_KEY_ENV" '{backend: $backend, private_key_env: $env}')"
    ;;
  keystore)
    if [[ -z "$KEYSTORE_PATH" ]]; then
      echo "KEYSTORE_PATH or ETH_VAULT_DEPLOYER_KEYSTORE is required for SIGNER_BACKEND=keystore" >&2
      exit 1
    fi
    if [[ -z "$PASSWORD_FILE" ]]; then
      echo "PASSWORD_FILE or ETH_VAULT_DEPLOYER_PASSWORD_FILE is required for SIGNER_BACKEND=keystore" >&2
      exit 1
    fi
    test -f "$KEYSTORE_PATH"
    test -f "$PASSWORD_FILE"
    SIGNER_ARGS=(--keystore "$KEYSTORE_PATH" --password-file "$PASSWORD_FILE")
    SIGNER_RECORD="$(jq -n --arg backend "$SIGNER_BACKEND" --arg keystore "$KEYSTORE_PATH" --arg password_file "$PASSWORD_FILE" '{backend: $backend, keystore: $keystore, password_file: $password_file}')"
    ;;
  *)
    echo "invalid SIGNER_BACKEND=$SIGNER_BACKEND; expected private_key_env or keystore" >&2
    exit 1
    ;;
esac

mkdir -p "$RUN_DIR"
cd "$CONTRACT_ROOT"
DEPLOY_OUTPUT_TEXT="$RUN_DIR/deploy-output.txt"

jq -n \
  --arg owner "$OWNER" \
  --arg treasury "$TREASURY" \
  --arg universal_router "$UNIVERSAL_ROUTER" \
  --arg permit2 "$PERMIT2" \
  --arg hooks_allowed "$HOOKS_ALLOWED" \
  --argjson signer "$SIGNER_RECORD" \
  '{
    schema: "constructor_args_v1",
    owner: $owner,
    treasury: $treasury,
    universal_router: $universal_router,
    permit2: $permit2,
    hooks_allowed: ($hooks_allowed == "true"),
    signer: $signer
  }' > "$RUN_DIR/constructor-args.json"

set +x
DEPLOY_OUTPUT="$(
  forge create \
    src/v4/UniswapV4TradingVault.sol:UniswapV4TradingVault \
    --rpc-url "$MAINNET_RPC_URL" \
    --broadcast \
    "${SIGNER_ARGS[@]}" \
    --constructor-args "$OWNER" "$TREASURY" "$UNIVERSAL_ROUTER" "$PERMIT2" "$HOOKS_ALLOWED" \
    | tee "$DEPLOY_OUTPUT_TEXT"
)"
printf '%s\n' "$DEPLOY_OUTPUT"

DEPLOYER="$(printf '%s\n' "$DEPLOY_OUTPUT" | awk -F': ' '/^Deployer:/ {print $2; exit}')"
DEPLOYED_TO="$(printf '%s\n' "$DEPLOY_OUTPUT" | awk -F': ' '/^Deployed to:/ {print $2; exit}')"
TRANSACTION_HASH="$(printf '%s\n' "$DEPLOY_OUTPUT" | awk -F': ' '/^Transaction hash:/ {print $2; exit}')"

if [[ -z "$DEPLOYER" || -z "$DEPLOYED_TO" || -z "$TRANSACTION_HASH" ]]; then
  echo "failed to parse deploy output; see $DEPLOY_OUTPUT_TEXT" >&2
  exit 1
fi

cast receipt "$TRANSACTION_HASH" --rpc-url "$MAINNET_RPC_URL" --json > "$RUN_DIR/deploy-receipt.json"

jq -n \
  --arg schema "deploy_output_v1" \
  --arg deployer "$DEPLOYER" \
  --arg contract_address "$DEPLOYED_TO" \
  --arg transaction_hash "$TRANSACTION_HASH" \
  --arg raw_output "$DEPLOY_OUTPUT_TEXT" \
  '{
    schema: $schema,
    deployer: $deployer,
    contract_address: $contract_address,
    transaction_hash: $transaction_hash,
    raw_output: $raw_output
  }' > "$RUN_DIR/deploy-output.json"

echo "run_dir=$RUN_DIR"
