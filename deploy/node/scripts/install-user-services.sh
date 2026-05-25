#!/usr/bin/env bash
set -euo pipefail

NODE_DEPLOY_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
DEPLOY_DIR=$(cd "$NODE_DEPLOY_DIR/.." && pwd)
ETH_DIR=$(cd "$DEPLOY_DIR/.." && pwd)
USER_UNIT_SRC_DIR="$DEPLOY_DIR/systemd/user"
USER_SYSTEMD_DIR=${USER_SYSTEMD_DIR:-$HOME/.config/systemd/user}
RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
ENABLE_NODE_SERVICES=${ENABLE_NODE_SERVICES:-0}
ENABLE_MEMPOOL_SIGNAL_DETECTOR_SERVICE=${ENABLE_MEMPOOL_SIGNAL_DETECTOR_SERVICE:-1}
BUILD_MEMPOOL_SIGNAL_DETECTOR=${BUILD_MEMPOOL_SIGNAL_DETECTOR:-1}

mkdir -p "$USER_SYSTEMD_DIR"

if [ "$ENABLE_MEMPOOL_SIGNAL_DETECTOR_SERVICE" = "1" ] && [ "$BUILD_MEMPOOL_SIGNAL_DETECTOR" = "1" ]; then
  cargo --manifest-path "$ETH_DIR/Cargo.toml" build --release -p mempool_processor --bin mempool_signal_detector
fi

if [ "$ENABLE_NODE_SERVICES" = "1" ]; then
  ln -sfn "$USER_UNIT_SRC_DIR/reth.service" "$USER_SYSTEMD_DIR/reth.service"
  ln -sfn "$USER_UNIT_SRC_DIR/lighthouse-beacon.service" "$USER_SYSTEMD_DIR/lighthouse-beacon.service"
fi
ln -sfn "$USER_UNIT_SRC_DIR/eth-mempool-signal-detector.service" "$USER_SYSTEMD_DIR/eth-mempool-signal-detector.service"
ln -sfn "$USER_UNIT_SRC_DIR/eth-alpha-live-backtest.service" "$USER_SYSTEMD_DIR/eth-alpha-live-backtest.service"
ln -sfn "$USER_UNIT_SRC_DIR/eth-alpha-live-real-trading.service" "$USER_SYSTEMD_DIR/eth-alpha-live-real-trading.service"
ln -sfn "$USER_UNIT_SRC_DIR/eth-alpha-live-backtest.target" "$USER_SYSTEMD_DIR/eth-alpha-live-backtest.target"
ln -sfn "$USER_UNIT_SRC_DIR/eth-alpha-live-real-trading.target" "$USER_SYSTEMD_DIR/eth-alpha-live-real-trading.target"

XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user daemon-reload

if [ "$ENABLE_NODE_SERVICES" = "1" ]; then
  XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user enable reth.service lighthouse-beacon.service
  echo "linked and enabled node user services"
else
  echo "skipped node user services; system reth/lighthouse/chain-server units are expected"
fi

if [ "$ENABLE_MEMPOOL_SIGNAL_DETECTOR_SERVICE" = "1" ]; then
  XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user enable eth-mempool-signal-detector.service
  echo "linked and enabled Ethereum mempool signal detector user service"
else
  echo "linked Ethereum mempool signal detector user service without enabling autostart"
fi

echo "start with:"
if [ "$ENABLE_NODE_SERVICES" = "1" ]; then
  echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start reth.service"
  echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start lighthouse-beacon.service"
fi
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start eth-mempool-signal-detector.service"
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start eth-alpha-live-backtest.target"
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start eth-alpha-live-real-trading.target"
