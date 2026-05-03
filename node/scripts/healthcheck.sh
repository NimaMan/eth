#!/usr/bin/env bash
set -euo pipefail

echo "reth version:"
/home/nima/storage/samsung8tb/ethereum/bin/reth --version

echo
echo "lighthouse version:"
/home/nima/storage/samsung8tb/ethereum/bin/lighthouse --version

echo
echo "reth eth_syncing:"
curl -s http://127.0.0.1:8545 \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"eth_syncing","params":[]}' || true

echo
echo
echo "reth block number:"
curl -s http://127.0.0.1:8545 \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}' || true

echo
echo
echo "lighthouse syncing:"
curl -s http://127.0.0.1:5052/eth/v1/node/syncing || true
echo
