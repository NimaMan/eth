#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"
CONTRACT_ROOT="$ETH_ROOT/solidity/baygus-executor/contracts"
ARTIFACT="$CONTRACT_ROOT/out/UniswapV2TradingVault.sol/UniswapV2TradingVault.json"

mkdir -p "$RUN_DIR"
cd "$ETH_ROOT"

git rev-parse HEAD > "$RUN_DIR/git-revision.txt"
git status --short > "$RUN_DIR/git-status.txt"

cd "$CONTRACT_ROOT"
forge fmt --check | tee "$RUN_DIR/forge-fmt.txt"
forge build | tee "$RUN_DIR/forge-build.txt"
forge test | tee "$RUN_DIR/forge-test.txt"

cd "$ETH_ROOT"
cargo test -p tx_simulator tx_builders::protocols::uniswap_v2_trading_vault --lib | tee "$RUN_DIR/cargo-test-tx-simulator.txt"
cargo test -p eth_live_trading | tee "$RUN_DIR/cargo-test-eth-live-trading.txt"

BYTECODE="$(jq -r '.bytecode.object' "$ARTIFACT")"
DEPLOYED_BYTECODE="$(jq -r '.deployedBytecode.object' "$ARTIFACT")"

jq -n \
  --arg artifact "$ARTIFACT" \
  --arg bytecode_hash "$(cast keccak "$BYTECODE")" \
  --arg deployed_bytecode_hash "$(cast keccak "$DEPLOYED_BYTECODE")" \
  --arg abi_hash "$(jq -c '.abi' "$ARTIFACT" | cast keccak)" \
  '{
    schema: "artifact_hashes_v1",
    artifact: $artifact,
    bytecode_hash: $bytecode_hash,
    deployed_bytecode_hash: $deployed_bytecode_hash,
    abi_hash: $abi_hash
  }' > "$RUN_DIR/artifact-hashes.json"

echo "run_dir=$RUN_DIR"
echo "artifact_hashes=$RUN_DIR/artifact-hashes.json"
