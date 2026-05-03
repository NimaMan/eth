#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=load-config.sh
. "$SCRIPT_DIR/load-config.sh"

ETH_NODE_ROOT=${ETH_NODE_ROOT:-/home/nima/storage/samsung8tb/ethereum}
RETH_HTTP_RPC=${RETH_HTTP_RPC:-http://127.0.0.1:8545}
LIGHTHOUSE_HTTP_API=${LIGHTHOUSE_HTTP_API:-http://127.0.0.1:5052}

echo "reth version:"
"$ETH_NODE_ROOT/bin/reth" --version

echo
echo "lighthouse version:"
"$ETH_NODE_ROOT/bin/lighthouse" --version

echo
echo "reth eth_syncing:"
curl -s "$RETH_HTTP_RPC" \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"eth_syncing","params":[]}' || true

echo
echo
echo "reth block number:"
curl -s "$RETH_HTTP_RPC" \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}' || true

echo
echo
echo "lighthouse syncing:"
curl -s "$LIGHTHOUSE_HTTP_API/eth/v1/node/syncing" || true
echo
