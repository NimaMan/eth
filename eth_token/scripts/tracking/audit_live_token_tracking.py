#!/usr/bin/env python3
"""Audit live token tracker pool discovery against chain pool-creation logs."""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request
from collections import Counter
from dataclasses import dataclass
from typing import Any


PAIR_CREATED = "0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9"
POOL_CREATED_V3 = "0x783cca1c0412dd0d695e784568c96da2e9c22ff989357a2e8b1d9b2b4e6b7118"
INITIALIZE_V4 = "0xdd466e674ea557f56295e2d0218a125ea4b4f0f6f3307b95f85e6110838d6438"

V2_FACTORIES = {
    "0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f": "UNISWAP-V2",
    "0xc0aee478e3658e2610c5f7a4a2e1777ce9e4f2ac": "SUSHISWAP-V2",
    "0x1097053fd2ea711dad45caccc45eff7548fcb362": "PANCAKESWAP-V2",
    "0x115934131916c8b277dd010ee02de363c09d037c": "SHIBASWAP-V2",
    "0x43ec799eadd63848443e2347c49f5f52e8fe0f6f": "FRAXSWAP-V2",
}

V3_FACTORIES = {
    "0x1f98431c8ad98523631ae4a59f267346ea31f984": "UNISWAP-V3",
    "0xbaceb8ec6b9355dfc0269c18bac9d6e2bdc29c4f": "SUSHISWAP-V3",
    "0x0bfbcf9fa4f9c56b0f40a671ad40e0805a091865": "PANCAKESWAP-V3",
}

V4_MANAGERS = {
    "0x000000000004444c5dc75cb358380d2e3de08a90": "UNISWAP-V4",
}


@dataclass(frozen=True)
class ExpectedPoolLink:
    token: str
    pool: str
    protocol: str
    kind: str
    block: int
    tx_hash: str
    log_index: int
    denom: str


def load_json_url(url: str) -> Any:
    with urllib.request.urlopen(url, timeout=30) as response:
        return json.load(response)


def rpc_call(rpc_url: str, method: str, params: list[Any], request_id: int = 1) -> Any:
    payload = json.dumps(
        {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
    ).encode()
    request = urllib.request.Request(
        rpc_url,
        data=payload,
        headers={"content-type": "application/json"},
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            result = json.load(response)
    except urllib.error.URLError as error:
        raise SystemExit(f"RPC request failed: {error}") from error

    if "error" in result:
        raise SystemExit(f"RPC error for {method}: {result['error']}")
    return result["result"]


def normalize(value: str | None) -> str:
    return (value or "").strip().lower()


def hex_int(value: str | None) -> int:
    return int(value or "0x0", 16)


def topic_address(topic: str) -> str:
    return "0x" + topic[-40:].lower()


def word_address(data: str, word_index: int) -> str:
    body = data[2:] if data.startswith("0x") else data
    start = word_index * 64
    word = body[start : start + 64]
    return "0x" + word[-40:].lower()


def request_logs(
    rpc_url: str,
    start_block: int,
    end_block: int,
    address: str,
    topic0: str,
    chunk_size: int,
) -> list[dict[str, Any]]:
    logs: list[dict[str, Any]] = []
    block = start_block
    request_id = 1
    while block <= end_block:
        chunk_end = min(end_block, block + chunk_size - 1)
        params = [
            {
                "fromBlock": hex(block),
                "toBlock": hex(chunk_end),
                "address": address,
                "topics": [topic0],
            }
        ]
        logs.extend(rpc_call(rpc_url, "eth_getLogs", params, request_id))
        request_id += 1
        block = chunk_end + 1
    return logs


def fetch_creation_logs(
    rpc_url: str, start_block: int, end_block: int, chunk_size: int
) -> list[dict[str, Any]]:
    logs: list[dict[str, Any]] = []
    for address in V2_FACTORIES:
        for log in request_logs(rpc_url, start_block, end_block, address, PAIR_CREATED, chunk_size):
            log["protocol"] = V2_FACTORIES[normalize(log["address"])]
            log["kind"] = "v2"
            logs.append(log)
    for address in V3_FACTORIES:
        for log in request_logs(rpc_url, start_block, end_block, address, POOL_CREATED_V3, chunk_size):
            log["protocol"] = V3_FACTORIES[normalize(log["address"])]
            log["kind"] = "v3"
            logs.append(log)
    for address in V4_MANAGERS:
        for log in request_logs(rpc_url, start_block, end_block, address, INITIALIZE_V4, chunk_size):
            log["protocol"] = V4_MANAGERS[normalize(log["address"])]
            log["kind"] = "v4"
            logs.append(log)
    return sorted(logs, key=lambda item: (hex_int(item["blockNumber"]), hex_int(item["logIndex"])))


def expected_links(
    logs: list[dict[str, Any]], tracked_tokens: set[str]
) -> list[ExpectedPoolLink]:
    expected: list[ExpectedPoolLink] = []
    for log in logs:
        kind = log["kind"]
        protocol = log["protocol"]
        block = hex_int(log["blockNumber"])
        log_index = hex_int(log["logIndex"])
        tx_hash = normalize(log.get("transactionHash"))
        topics = log["topics"]
        data = log.get("data", "0x")

        if kind == "v2":
            if len(topics) != 3:
                continue
            token0 = topic_address(topics[1])
            token1 = topic_address(topics[2])
            pool = word_address(data, 0)
        elif kind == "v3":
            if len(topics) != 4:
                continue
            token0 = topic_address(topics[1])
            token1 = topic_address(topics[2])
            pool = word_address(data, 1)
        else:
            if len(topics) != 4:
                continue
            token0 = topic_address(topics[2])
            token1 = topic_address(topics[3])
            pool = f"{normalize(log['address'])}#{normalize(topics[1])}"

        if token0 in tracked_tokens:
            expected.append(
                ExpectedPoolLink(token0, pool, protocol, kind, block, tx_hash, log_index, token1)
            )
        if token1 in tracked_tokens:
            expected.append(
                ExpectedPoolLink(token1, pool, protocol, kind, block, tx_hash, log_index, token0)
            )

    return expected


def current_links(pools: list[dict[str, Any]]) -> set[tuple[str, str]]:
    links = set()
    for pool in pools:
        token = normalize(pool.get("token_address"))
        pool_id = normalize(pool.get("pool_address"))
        if token and pool_id:
            links.add((token, pool_id))
    return links


def counts_by(items: list[Any], attr: str) -> dict[str, int]:
    return dict(sorted(Counter(getattr(item, attr) for item in items).items()))


def log_counts_by(logs: list[dict[str, Any]], key: str) -> dict[str, int]:
    return dict(sorted(Counter(log[key] for log in logs).items()))


def format_row(values: list[Any], widths: list[int]) -> str:
    return "  ".join(str(value).ljust(width) for value, width in zip(values, widths))


def print_examples(
    title: str,
    rows: list[ExpectedPoolLink],
    token_meta: dict[str, dict[str, Any]],
    limit: int,
) -> None:
    if not rows or limit <= 0:
        return
    print(f"\n{title}")
    headers = ["kind", "protocol", "block", "token", "symbol", "pool", "denom", "tx", "log"]
    widths = [5, 14, 8, 42, 12, 76, 42, 18, 5]
    print(format_row(headers, widths))
    print(format_row(["-" * min(width, 10) for width in widths], widths))
    for row in rows[:limit]:
        meta = token_meta.get(row.token, {})
        symbol = meta.get("symbol") or ""
        values = [
            row.kind,
            row.protocol,
            row.block,
            row.token,
            symbol[:12],
            row.pool,
            row.denom,
            row.tx_hash[:18],
            row.log_index,
        ]
        print(format_row(values, widths))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--live-api",
        default="http://127.0.0.1:8765/eth/tokens/api/live-token-tracker",
    )
    parser.add_argument("--rpc-url", default="http://127.0.0.1:8545")
    parser.add_argument("--start-block", type=int)
    parser.add_argument("--end-block", type=int)
    parser.add_argument("--chunk-size", type=int, default=2_000)
    parser.add_argument("--examples", type=int, default=20)
    parser.add_argument("--json", action="store_true", dest="as_json")
    args = parser.parse_args()

    status_payload = load_json_url(f"{args.live_api}/status")
    token_payload = load_json_url(f"{args.live_api}/tokens")
    pool_payload = load_json_url(f"{args.live_api}/pools")

    progress = status_payload.get("progress", status_payload)
    tokens = token_payload.get("tokens", [])
    pools = pool_payload.get("pools", [])
    start_block = args.start_block or progress["warmup_start_block"]
    end_block = args.end_block or progress["current_block"]
    live_start = progress.get("warmup_end_block", 0) + 1

    token_meta = {normalize(token.get("contract_address")): token for token in tokens}
    tracked_tokens = set(token_meta)
    current = current_links(pools)
    logs = fetch_creation_logs(args.rpc_url, start_block, end_block, args.chunk_size)
    expected = expected_links(logs, tracked_tokens)
    expected_keys = {(link.token, link.pool) for link in expected}

    missing = [link for link in expected if (link.token, link.pool) not in current]
    missing_live = [link for link in missing if link.block >= live_start]
    extra_current = sorted(current - expected_keys)

    summary = {
        "range": {"start": start_block, "end": end_block, "live_start": live_start},
        "progress": {
            "current_block": progress.get("current_block"),
            "tracked_tokens": progress.get("tracked_tokens"),
            "tracked_pools": progress.get("tracked_pools"),
            "tracked_v2_pools": progress.get("tracked_v2_pools"),
            "tracked_v3_pools": progress.get("tracked_v3_pools"),
            "tracked_v4_pools": progress.get("tracked_v4_pools"),
            "discovered_v2_pools_unique": progress.get("discovered_v2_pools_unique"),
            "discovered_v3_pools_unique": progress.get("discovered_v3_pools_unique"),
            "discovered_v4_pools_unique": progress.get("discovered_v4_pools_unique"),
            "retention_dropped_v2_pools": progress.get("retention_dropped_v2_pools"),
            "transaction_failures": progress.get("transaction_failures"),
        },
        "creation_logs_by_kind": log_counts_by(logs, "kind"),
        "creation_logs_by_protocol": log_counts_by(logs, "protocol"),
        "expected_links_by_kind": counts_by(expected, "kind"),
        "missing_links_by_kind": counts_by(missing, "kind"),
        "missing_live_links_by_kind": counts_by(missing_live, "kind"),
        "expected_links": len(expected),
        "current_links": len(current),
        "missing_links": len(missing),
        "missing_live_links": len(missing_live),
        "extra_current_links": len(extra_current),
    }

    if args.as_json:
        output = {
            "summary": summary,
            "missing_examples": [link.__dict__ for link in missing[: args.examples]],
            "missing_live_examples": [link.__dict__ for link in missing_live[: args.examples]],
            "extra_current_examples": extra_current[: args.examples],
        }
        print(json.dumps(output, indent=2, sort_keys=True))
        return 0

    print("Live token tracking pool-discovery audit")
    print(f"range: {start_block} - {end_block}; live tail starts: {live_start}")
    print(f"tracked tokens: {len(tracked_tokens)}; current API token-pool links: {len(current)}")
    print(f"creation logs by kind: {summary['creation_logs_by_kind']}")
    print(f"expected tracked token-pool links by kind: {summary['expected_links_by_kind']}")
    print(f"missing current links by kind: {summary['missing_links_by_kind']}")
    print(f"missing live-tail links by kind: {summary['missing_live_links_by_kind']}")
    print(f"current links without a creation log in range: {len(extra_current)}")
    print(f"progress: {json.dumps(summary['progress'], sort_keys=True)}")

    print_examples("Missing examples", missing, token_meta, args.examples)
    print_examples("Missing live-tail examples", missing_live, token_meta, args.examples)
    return 0


if __name__ == "__main__":
    sys.exit(main())
