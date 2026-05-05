#!/usr/bin/env bash
set -euo pipefail

REPO_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
USER_SYSTEMD_DIR=${USER_SYSTEMD_DIR:-$HOME/.config/systemd/user}
RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
ENABLE_NODE_SERVICES=${ENABLE_NODE_SERVICES:-1}
ENABLE_LIVE_PROCESSOR_SERVICE=${ENABLE_LIVE_PROCESSOR_SERVICE:-1}

mkdir -p "$USER_SYSTEMD_DIR"

ln -sfn "$REPO_DIR/systemd/user/reth.service" "$USER_SYSTEMD_DIR/reth.service"
ln -sfn "$REPO_DIR/systemd/user/lighthouse-beacon.service" "$USER_SYSTEMD_DIR/lighthouse-beacon.service"
ln -sfn "$REPO_DIR/systemd/user/eth-rust-live-block-processor.service" "$USER_SYSTEMD_DIR/eth-rust-live-block-processor.service"

XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user daemon-reload

if [ "$ENABLE_NODE_SERVICES" = "1" ]; then
  XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user enable reth.service lighthouse-beacon.service
  echo "linked and enabled node user services"
else
  echo "linked node user services without enabling autostart"
fi

if [ "$ENABLE_LIVE_PROCESSOR_SERVICE" = "1" ]; then
  XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user enable eth-rust-live-block-processor.service
  echo "linked and enabled Rust live block processor user service"
else
  echo "linked Rust live block processor user service without enabling autostart"
fi

echo "start with:"
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start reth.service"
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start lighthouse-beacon.service"
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start eth-rust-live-block-processor.service"
