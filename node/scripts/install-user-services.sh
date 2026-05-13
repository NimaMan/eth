#!/usr/bin/env bash
set -euo pipefail

REPO_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ETH_DIR=$(cd "$REPO_DIR/.." && pwd)
USER_SYSTEMD_DIR=${USER_SYSTEMD_DIR:-$HOME/.config/systemd/user}
RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
ENABLE_NODE_SERVICES=${ENABLE_NODE_SERVICES:-1}
ENABLE_CHAIN_SERVER_SERVICE=${ENABLE_CHAIN_SERVER_SERVICE:-1}
BUILD_CHAIN_SERVER=${BUILD_CHAIN_SERVER:-1}

mkdir -p "$USER_SYSTEMD_DIR"

if [ "$ENABLE_CHAIN_SERVER_SERVICE" = "1" ] && [ "$BUILD_CHAIN_SERVER" = "1" ]; then
  cargo --manifest-path "$ETH_DIR/Cargo.toml" build --release -p eth_chain_server
fi

ln -sfn "$REPO_DIR/systemd/user/reth.service" "$USER_SYSTEMD_DIR/reth.service"
ln -sfn "$REPO_DIR/systemd/user/lighthouse-beacon.service" "$USER_SYSTEMD_DIR/lighthouse-beacon.service"
ln -sfn "$REPO_DIR/systemd/user/eth-chain-server.service" "$USER_SYSTEMD_DIR/eth-chain-server.service"

XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user daemon-reload

if [ "$ENABLE_NODE_SERVICES" = "1" ]; then
  XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user enable reth.service lighthouse-beacon.service
  echo "linked and enabled node user services"
else
  echo "linked node user services without enabling autostart"
fi

if [ "$ENABLE_CHAIN_SERVER_SERVICE" = "1" ]; then
  XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user enable eth-chain-server.service
  echo "linked and enabled Ethereum chain server user service"
else
  echo "linked Ethereum chain server user service without enabling autostart"
fi

echo "start with:"
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start reth.service"
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start lighthouse-beacon.service"
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start eth-chain-server.service"
