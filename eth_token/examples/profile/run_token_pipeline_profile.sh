#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
ETH_ROOT=$(cd -- "$SCRIPT_DIR/../../.." && pwd)

START_BLOCK=${START_BLOCK:-25052270}
END_BLOCK=${END_BLOCK:-25059269}
TOKEN_PROFILE_BIND=${TOKEN_PROFILE_BIND:-127.0.0.1:8766}
TOKEN_PROFILE_LOG_DIR=${TOKEN_PROFILE_LOG_DIR:-$ETH_ROOT/logs/eth_chain_server_profile_post_block}
TOKEN_PROFILE_LOG_RUN_ID=${TOKEN_PROFILE_LOG_RUN_ID:-profile_${START_BLOCK}_${END_BLOCK}_$(date +%s)_$$}
TOKEN_PROFILE_API_PREFIX=${TOKEN_PROFILE_API_PREFIX:-/eth/tokens/api}
TOKEN_PROFILE_DEFAULT_BLOCKS=${TOKEN_PROFILE_DEFAULT_BLOCKS:-7000}
BASE_CONFIG=${TOKEN_PROFILE_BASE_CONFIG:-$ETH_ROOT/config.env}
PROFILE_CONFIG=${TOKEN_PROFILE_CONFIG:-/tmp/eth_chain_server_profile_post_block.env}

python3 - "$BASE_CONFIG" "$PROFILE_CONFIG" "$TOKEN_PROFILE_BIND" "$TOKEN_PROFILE_LOG_DIR" "$TOKEN_PROFILE_LOG_RUN_ID" "$TOKEN_PROFILE_DEFAULT_BLOCKS" <<'PY'
from pathlib import Path
import sys

base = Path(sys.argv[1])
dest = Path(sys.argv[2])
bind = sys.argv[3]
log_dir = sys.argv[4]
log_run_id = sys.argv[5]
default_blocks = sys.argv[6]

overrides = {
    "CHAIN_SERVER_BIND": bind,
    "CHAIN_SERVER_LOG_DIR": log_dir,
    "CHAIN_SERVER_LOG_RUN_ID": log_run_id,
    "CHAIN_SERVER_DEFAULT_BLOCKS": str(int(default_blocks)),
    "CHAIN_SERVER_AUTO_START_LIVE": "false",
}

seen = set()
lines = []
for raw in base.read_text().splitlines():
    key = raw.split("=", 1)[0].strip() if "=" in raw else ""
    if key in overrides:
        lines.append(f"{key}={overrides[key]}")
        seen.add(key)
    else:
        lines.append(raw)

for key, value in overrides.items():
    if key not in seen:
        lines.append(f"{key}={value}")

dest.write_text("\n".join(lines) + "\n")
PY

cargo build --manifest-path "$ETH_ROOT/Cargo.toml" -p eth_chain_server --release

"$ETH_ROOT/target/release/eth_chain_server" --config "$PROFILE_CONFIG" &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT

python3 - "$TOKEN_PROFILE_BIND" "$TOKEN_PROFILE_API_PREFIX" "$START_BLOCK" "$END_BLOCK" <<'PY'
import json
import sys
import time
import urllib.error
import urllib.request

bind, api_prefix, start_block, end_block = sys.argv[1], sys.argv[2].rstrip("/"), int(sys.argv[3]), int(sys.argv[4])
base = f"http://{bind}{api_prefix}"

for _ in range(120):
    try:
        urllib.request.urlopen(f"{base}/health", timeout=1).read()
        break
    except Exception:
        time.sleep(0.5)
else:
    raise SystemExit("server did not become healthy")

payload = json.dumps(
    {
        "start_block": start_block,
        "end_block": end_block,
        "replace_active": True,
    }
).encode()
request = urllib.request.Request(
    f"{base}/runs",
    data=payload,
    headers={"content-type": "application/json"},
    method="POST",
)
with urllib.request.urlopen(request, timeout=10) as response:
    run = json.load(response)

run_id = run["id"]
print(f"started run_id={run_id} range={start_block}-{end_block}", flush=True)

started = time.time()
while True:
    with urllib.request.urlopen(f"{base}/runs/{run_id}/progress", timeout=10) as response:
        progress = json.load(response)
    status = progress["status"]
    print(
        "t={:.0f}s status={} processed={}/{} current={} tokens={} pools={} last_apply_ms={}".format(
            time.time() - started,
            status,
            progress["blocks_processed"],
            progress["total_blocks"],
            progress["current_block"],
            progress["tracked_tokens"],
            progress["indexed_pools"],
            progress["last_block_token_apply_ms"],
        ),
        flush=True,
    )
    if status in {"completed", "failed", "stopped"}:
        if status != "completed":
            raise SystemExit(f"run ended with status={status}")
        print(f"completed run_id={run_id}", flush=True)
        break
    time.sleep(60)
PY

RUN_ID=$(python3 - "$TOKEN_PROFILE_BIND" "$TOKEN_PROFILE_API_PREFIX" <<'PY'
import json
import sys
import urllib.request

bind, api_prefix = sys.argv[1], sys.argv[2].rstrip("/")
with urllib.request.urlopen(f"http://{bind}{api_prefix}/runs/active", timeout=10) as response:
    print(json.load(response)["id"])
PY
)

LOG_FILE="$TOKEN_PROFILE_LOG_DIR/$TOKEN_PROFILE_LOG_RUN_ID/token_pipeline_profile.jsonl"
CSV_FILE="/tmp/token_pipeline_profile_${RUN_ID}_${START_BLOCK}_${END_BLOCK}.csv"

python3 "$ETH_ROOT/eth_chain_server/scripts/token_pipeline_profile_summary.py" \
    "$LOG_FILE" \
    --run-id "$RUN_ID" \
    --csv "$CSV_FILE" \
    --top 20

echo "csv=$CSV_FILE"
