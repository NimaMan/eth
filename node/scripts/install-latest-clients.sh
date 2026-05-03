#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=load-config.sh
. "$SCRIPT_DIR/load-config.sh"

ETH_NODE_ROOT=${ETH_NODE_ROOT:-/home/nima/storage/samsung8tb/ethereum}
BIN_DIR="$ETH_NODE_ROOT/bin"
DOWNLOAD_DIR="$ETH_NODE_ROOT/downloads"

require() {
  command -v "$1" >/dev/null || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

install_asset() {
  local repo=$1
  local asset_prefix=$2
  local target_link=$3
  local latest_json asset tag url digest archive install_dir

  latest_json="$DOWNLOAD_DIR/${asset_prefix}-latest.json"
  curl -fsSL "https://api.github.com/repos/${repo}/releases/latest" -o "$latest_json"

  tag=$(jq -r '.tag_name' "$latest_json")
  asset="${asset_prefix}-${tag}-x86_64-unknown-linux-gnu.tar.gz"
  url=$(jq -r --arg name "$asset" '.assets[] | select(.name==$name) | .browser_download_url' "$latest_json")
  digest=$(jq -r --arg name "$asset" '.assets[] | select(.name==$name) | .digest' "$latest_json" | sed 's/^sha256://')

  if [ -z "$url" ] || [ "$url" = "null" ]; then
    echo "could not find release asset $asset for $repo" >&2
    exit 1
  fi

  archive="$DOWNLOAD_DIR/$asset"
  install_dir="$BIN_DIR/${asset_prefix}-${tag}"

  curl -fL "$url" -o "$archive"
  printf '%s  %s\n' "$digest" "$archive" | sha256sum -c -

  rm -rf "$install_dir"
  mkdir -p "$install_dir"
  tar -xzf "$archive" -C "$install_dir"
  ln -sfn "$install_dir/$asset_prefix" "$BIN_DIR/$target_link"

  "$BIN_DIR/$target_link" --version
}

require curl
require jq
require sha256sum
require tar

mkdir -p "$BIN_DIR" "$DOWNLOAD_DIR"

install_asset paradigmxyz/reth reth reth
install_asset sigp/lighthouse lighthouse lighthouse
