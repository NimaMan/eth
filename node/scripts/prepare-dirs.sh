#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=load-config.sh
. "$SCRIPT_DIR/load-config.sh"

ETH_NODE_ROOT=${ETH_NODE_ROOT:-/home/nima/storage/samsung8tb/ethereum}
LIGHTHOUSE_DATADIR=${LIGHTHOUSE_DATADIR:-/home/nima/.lighthouse}
JWT_PATH=${JWT_PATH:-$ETH_NODE_ROOT/jwt/jwt.hex}

mkdir -p \
  "$ETH_NODE_ROOT/bin" \
  "$ETH_NODE_ROOT/downloads" \
  "$ETH_NODE_ROOT/jwt" \
  "$ETH_NODE_ROOT/reth" \
  "$LIGHTHOUSE_DATADIR"

if ! findmnt -T "$ETH_NODE_ROOT" >/dev/null; then
  echo "runtime root is not on a mounted filesystem: $ETH_NODE_ROOT" >&2
  exit 1
fi

if ! findmnt -T "$LIGHTHOUSE_DATADIR" >/dev/null; then
  echo "lighthouse datadir is not on a mounted filesystem: $LIGHTHOUSE_DATADIR" >&2
  exit 1
fi

if [ ! -s "$JWT_PATH" ]; then
  umask 077
  openssl rand -hex 32 > "$JWT_PATH"
fi

chmod 700 "$ETH_NODE_ROOT/jwt"
chmod 600 "$JWT_PATH"

echo "prepared $ETH_NODE_ROOT"
echo "lighthouse datadir: $LIGHTHOUSE_DATADIR"
echo "jwt: $JWT_PATH"
