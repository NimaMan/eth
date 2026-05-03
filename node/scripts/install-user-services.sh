#!/usr/bin/env bash
set -euo pipefail

REPO_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
USER_SYSTEMD_DIR=${USER_SYSTEMD_DIR:-$HOME/.config/systemd/user}
RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}

mkdir -p "$USER_SYSTEMD_DIR"

ln -sfn "$REPO_DIR/systemd/user/reth.service" "$USER_SYSTEMD_DIR/reth.service"
ln -sfn "$REPO_DIR/systemd/user/lighthouse-beacon.service" "$USER_SYSTEMD_DIR/lighthouse-beacon.service"

XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user daemon-reload
XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user enable reth.service lighthouse-beacon.service

echo "linked and enabled user services"
echo "start with:"
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start reth.service"
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR systemctl --user start lighthouse-beacon.service"
