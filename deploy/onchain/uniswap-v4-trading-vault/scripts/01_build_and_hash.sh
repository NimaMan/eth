#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ETH_ROOT="$(cd "$DEPLOY_DIR/../../.." && pwd)"
RUN_DIR="${RUN_DIR:-$DEPLOY_DIR/runs/$(date -u +%Y%m%d-%H%M%SZ)}"
CONTRACT_ROOT="$ETH_ROOT/solidity/baygus-executor/contracts"
ARTIFACT="$CONTRACT_ROOT/../out/UniswapV4TradingVault.sol/UniswapV4TradingVault.json"

mkdir -p "$RUN_DIR"
cd "$ETH_ROOT"

git rev-parse HEAD > "$RUN_DIR/git-revision.txt"
git status --short > "$RUN_DIR/git-status.txt"

cd "$CONTRACT_ROOT"
forge fmt --check | tee "$RUN_DIR/forge-fmt.txt"
forge build | tee "$RUN_DIR/forge-build.txt"
forge test --match-path 'test/v4/*.sol' -vv | tee "$RUN_DIR/forge-test.txt"

cd "$ETH_ROOT"
cargo check -p tx_simulator --example uniswap_v4_trading_vault_candidate_rehearsal 2>&1 \
  | tee "$RUN_DIR/cargo-check-tx-simulator-v4.txt"
cargo test -p tx_simulator v4 --lib 2>&1 | tee "$RUN_DIR/cargo-test-tx-simulator-v4.txt"

BYTECODE="$(jq -r '.bytecode.object' "$ARTIFACT")"
DEPLOYED_BYTECODE="$(jq -r '.deployedBytecode.object' "$ARTIFACT")"

jq -n \
  --arg artifact "$ARTIFACT" \
  --arg bytecode_hash "$(cast keccak "$BYTECODE")" \
  --arg deployed_bytecode_hash "$(cast keccak "$DEPLOYED_BYTECODE")" \
  --arg abi_hash "$(jq -c '.abi' "$ARTIFACT" | cast keccak)" \
  --arg artifact_sha256 "$(sha256sum "$ARTIFACT" | awk '{print $1}')" \
  --arg source_sha256 "$(sha256sum "$ETH_ROOT/solidity/baygus-executor/contracts/src/v4/UniswapV4TradingVault.sol" | awk '{print $1}')" \
  '{
    schema: "artifact_hashes_v1",
    artifact: $artifact,
    bytecode_hash: $bytecode_hash,
    deployed_bytecode_hash: $deployed_bytecode_hash,
    abi_hash: $abi_hash,
    artifact_sha256: $artifact_sha256,
    source_sha256: $source_sha256
  }' > "$RUN_DIR/artifact-hashes.json"

echo "run_dir=$RUN_DIR"
echo "artifact_hashes=$RUN_DIR/artifact-hashes.json"
