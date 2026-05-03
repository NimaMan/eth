#!/usr/bin/env bash

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
ETH_REPO_ROOT=$(cd -- "$SCRIPT_DIR/../.." && pwd)
ETH_CONFIG_PATH=${ETH_CONFIG_PATH:-$ETH_REPO_ROOT/config.env}

if [ -f "$ETH_CONFIG_PATH" ]; then
  while IFS='=' read -r key value; do
    key=${key#"${key%%[![:space:]]*}"}
    key=${key%"${key##*[![:space:]]}"}
    value=${value#"${value%%[![:space:]]*}"}
    value=${value%"${value##*[![:space:]]}"}

    if [ -z "$key" ] || [ "${key#\#}" != "$key" ]; then
      continue
    fi

    value=${value#\"}
    value=${value%\"}
    value=${value#\'}
    value=${value%\'}

    if [ -z "${!key+x}" ]; then
      export "$key=$value"
    fi
  done < "$ETH_CONFIG_PATH"
fi
