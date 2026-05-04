#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=load-config.sh
. "$SCRIPT_DIR/load-config.sh"

ETH_NODE_ROOT=${ETH_NODE_ROOT:-/home/nima/storage/samsung8tb/ethereum}
RETH_DATADIR=${RETH_DATADIR:-$ETH_NODE_ROOT/reth}
RETH_CHAIN=${RETH_CHAIN:-mainnet}
RETH_DOWNLOAD_UNIT=${RETH_DOWNLOAD_UNIT:-reth-archive-download}
RETH_DOWNLOAD_UNIT=${RETH_DOWNLOAD_UNIT%.service}
RETH_DOWNLOAD_CONCURRENCY=${RETH_DOWNLOAD_CONCURRENCY:-8}
RETH_ARCHIVE_SNAPSHOT_MANIFEST_URL=${RETH_ARCHIVE_SNAPSHOT_MANIFEST_URL:-}
RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}

if [ ! -x "$ETH_NODE_ROOT/bin/reth" ]; then
  echo "reth binary is missing or not executable: $ETH_NODE_ROOT/bin/reth" >&2
  echo "run ./scripts/install-latest-clients.sh first" >&2
  exit 1
fi

mkdir -p "$RETH_DATADIR" "$ETH_NODE_ROOT/logs"

if XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user is-active --quiet "$RETH_DOWNLOAD_UNIT.service"; then
  echo "$RETH_DOWNLOAD_UNIT.service is already running"
  echo "follow logs with:"
  echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR journalctl --user -u $RETH_DOWNLOAD_UNIT.service -f"
  exit 0
fi

XDG_RUNTIME_DIR="$RUNTIME_DIR" systemctl --user stop lighthouse-beacon.service reth.service 2>/dev/null || true

cmd=(
  "$ETH_NODE_ROOT/bin/reth"
  download
  --chain "$RETH_CHAIN"
  --datadir "$RETH_DATADIR"
  --archive
  --non-interactive
  --download-concurrency "$RETH_DOWNLOAD_CONCURRENCY"
)

if [ -n "$RETH_ARCHIVE_SNAPSHOT_MANIFEST_URL" ]; then
  cmd+=(--manifest-url "$RETH_ARCHIVE_SNAPSHOT_MANIFEST_URL")
fi

XDG_RUNTIME_DIR="$RUNTIME_DIR" systemd-run --user \
  --unit="$RETH_DOWNLOAD_UNIT" \
  --description="Download Reth archive snapshot" \
  --property=WorkingDirectory="$HOME" \
  --collect \
  "${cmd[@]}"

echo "started $RETH_DOWNLOAD_UNIT.service"
echo "reth datadir: $RETH_DATADIR"
echo "follow logs with:"
echo "  XDG_RUNTIME_DIR=$RUNTIME_DIR journalctl --user -u $RETH_DOWNLOAD_UNIT.service -f"
