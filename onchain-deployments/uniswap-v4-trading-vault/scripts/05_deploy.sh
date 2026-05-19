#!/usr/bin/env bash
set -euo pipefail

echo "Refusing to deploy: Uniswap V4 trading vault is assessment-only."
echo "Enable this script only after audit/checklist.yaml is complete."
exit 1
