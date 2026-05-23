#!/usr/bin/env python3
"""Deployment review for alpha11 hold15 live chain-sim results.

The script intentionally uses only the public local HTTP surfaces:

- Asena backtest API for strategy/trade state.
- Ethereum JSON-RPC for exact mined block transaction fee data.

It does not mutate alpha_trading tables.
"""

from __future__ import annotations

import csv
import json
import os
import statistics
import time
import urllib.error
import urllib.request
from collections import defaultdict
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path
from typing import Any


ASENA_URL = os.environ.get("ASENA_URL", "http://127.0.0.1:40019").rstrip("/")
ETH_RPC_URL = os.environ.get("ETH_RPC_URL", "http://127.0.0.1:8545").rstrip("/")

RESULT_SET_ID = (
    "live-alpha11-live-univ2-lp30-pool-update-block-hold-sweep-chain-sim-20260521-105807"
)
RUN_ID = "alpha11-live-univ2-lp30-pool-update-block-hold-sweep-chain-sim-20260521-105807"
STRATEGY = "alpha11-live-univ2-lp30-pool-update-block-hold15"
API_BASE = f"{ASENA_URL}/eth/backtest/api"

HERE = Path(__file__).resolve().parent
OUT_DIR = HERE / "artifacts"

WEI_PER_GWEI = Decimal("1000000000")
WEI_PER_ETH = Decimal("1000000000000000000")
GWEI_1 = 1_000_000_000
GWEI_2 = 2_000_000_000
GWEI_5 = 5_000_000_000


def request_json(url: str) -> Any:
    req = urllib.request.Request(
        url,
        headers={"accept": "application/json", "user-agent": "alpha11-deploy-review"},
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{url}: HTTP {exc.code}: {body[:500]}") from exc


def rpc(method: str, params: list[Any]) -> Any:
    payload = json.dumps({"jsonrpc": "2.0", "method": method, "params": params, "id": 1}).encode(
        "utf-8"
    )
    req = urllib.request.Request(
        ETH_RPC_URL,
        data=payload,
        headers={"content-type": "application/json", "accept": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=60) as response:
        body = json.loads(response.read().decode("utf-8"))
    if "error" in body:
        raise RuntimeError(f"RPC {method} failed: {body['error']}")
    return body.get("result")


def api(path: str) -> Any:
    return request_json(f"{API_BASE}/{path.lstrip('/')}")


def api_optional(path: str) -> Any | None:
    try:
        return api(path)
    except Exception as exc:
        print(f"warning: optional API load failed for {path}: {exc}")
        return None


def h2i(value: Any) -> int:
    if value is None:
        return 0
    if isinstance(value, int):
        return value
    text = str(value)
    return int(text, 16) if text.startswith("0x") else int(text)


def dec(value: Any, default: str = "0") -> Decimal:
    if value is None or value == "":
        return Decimal(default)
    return Decimal(str(value))


def wei_to_gwei(value: int | Decimal) -> Decimal:
    return Decimal(value) / WEI_PER_GWEI


def wei_to_eth(value: int | Decimal) -> Decimal:
    return Decimal(value) / WEI_PER_ETH


def gwei_to_wei(value: Decimal | int | float | str) -> int:
    return int((Decimal(str(value)) * WEI_PER_GWEI).to_integral_value(rounding=ROUND_HALF_UP))


def priority_spend_eth(priority_wei: int, gas_used: int) -> Decimal:
    if gas_used <= 0:
        return Decimal("0")
    return Decimal(priority_wei) * Decimal(gas_used) / WEI_PER_ETH


def fmt_eth(value: Any, places: int = 6) -> str:
    value = dec(value)
    q = Decimal(10) ** -places
    return str(value.quantize(q, rounding=ROUND_HALF_UP))


def fmt_gwei(value: Any, places: int = 3) -> str:
    value = dec(value)
    q = Decimal(10) ** -places
    return str(value.quantize(q, rounding=ROUND_HALF_UP))


def fmt_pct(value: Any, places: int = 2) -> str:
    value = dec(value)
    q = Decimal(10) ** -places
    return str(value.quantize(q, rounding=ROUND_HALF_UP))


def short(value: str, head: int = 8, tail: int = 6) -> str:
    if not value:
        return ""
    if len(value) <= head + tail + 3:
        return value
    return f"{value[:head]}...{value[-tail:]}"


MEMPOOL_SOURCES = {"mempool_signal", "historical_mempool_signal"}
MINED_CHAIN_SOURCES = {"risk_atlas_mined_chain"}


def risk_source(risk: dict[str, Any]) -> str:
    payload = risk.get("payload") if isinstance(risk.get("payload"), dict) else {}
    return str(risk.get("source") or payload.get("source") or "")


def source_label(source: str) -> str:
    if source in MEMPOOL_SOURCES:
        return "mempool"
    if source in MINED_CHAIN_SOURCES:
        return "mined-chain"
    if source:
        return source
    return "unknown-source"


def source_qualified_kind(risk: dict[str, Any]) -> str:
    kind = str(risk.get("kind") or "")
    source = risk_source(risk)
    prefix = source_label(source)
    if kind == "lp_approval":
        return f"{prefix} lp_approval"
    if kind == "liquidity_removal":
        return f"{prefix} liquidity_removal"
    if kind == "mempool_liquidity_removal":
        return "mempool liquidity_removal"
    if kind == "trading_enabled":
        return f"{prefix} trading_enabled"
    return f"{prefix} {kind}" if kind else prefix


def risk_evidence_label(risk: dict[str, Any], include_pending_hash: bool = True) -> str:
    parts = [source_qualified_kind(risk)]
    relation = risk.get("position_relation")
    if relation:
        parts.append(str(relation))
    observed_block = risk.get("observed_block")
    if observed_block not in (None, ""):
        parts.append(f"block {observed_block}")
    pending_tx_hash = str(risk.get("pending_tx_hash") or "")
    if include_pending_hash and pending_tx_hash:
        parts.append(f"pending {short(pending_tx_hash)}")
    return " ".join(part for part in parts if part)


def risk_evidence_summary(risks: list[dict[str, Any]], include_pending_hash: bool = True) -> str:
    labels = []
    seen = set()
    for risk in sorted(
        risks,
        key=lambda item: (
            int(item.get("observed_block") or 0),
            str(item.get("kind") or ""),
            str(item.get("position_relation") or ""),
        ),
    ):
        label = risk_evidence_label(risk, include_pending_hash=include_pending_hash)
        if label and label not in seen:
            seen.add(label)
            labels.append(label)
    return "; ".join(labels)


def expected_exit_risk_kind(reason_code: str) -> str:
    if reason_code == "exit.lp_approval":
        return "lp_approval"
    if reason_code == "exit.mempool_liquidity_removal_signal":
        return "mempool_liquidity_removal"
    if reason_code == "exit.liquidity_removal":
        return "liquidity_removal"
    return ""


def sell_trigger_risk(detail: dict[str, Any]) -> dict[str, Any] | None:
    token = detail.get("token") or {}
    reason_code = str(token.get("sell_reason_code") or token.get("sell_reason") or "")
    expected_kind = expected_exit_risk_kind(reason_code)
    if not expected_kind:
        return None

    sell_block = token.get("sell_reason_block")
    sell_source = str(token.get("sell_reason_source") or "")
    candidates = [
        risk
        for risk in detail.get("risks") or []
        if str(risk.get("kind") or "") == expected_kind
    ]
    if not candidates:
        return None

    def score(risk: dict[str, Any]) -> tuple[int, int, int]:
        same_block = sell_block is not None and str(risk.get("observed_block")) == str(sell_block)
        same_source = sell_source != "" and risk_source(risk) == sell_source
        in_position = risk.get("position_relation") == "in_position"
        return (1 if same_block else 0, 1 if same_source else 0, 1 if in_position else 0)

    return max(candidates, key=score)


def quantile(sorted_values: list[int], fraction: float) -> int:
    if not sorted_values:
        return 0
    idx = round((len(sorted_values) - 1) * fraction)
    return sorted_values[idx]


def rank_for_tip(priorities_desc: list[int], tip_wei: int) -> tuple[int, int]:
    ahead = sum(1 for value in priorities_desc if value > tip_wei)
    equal = sum(1 for value in priorities_desc if value == tip_wei)
    return ahead + 1, equal


def required_tip_for_position(priorities_desc: list[int], target_position: int) -> int:
    if not priorities_desc:
        return 0
    if target_position <= 0:
        target_position = 1
    if target_position > len(priorities_desc):
        return 0
    return priorities_desc[target_position - 1] + 1


def block_fee_sample(block_number: int, cache: dict[int, dict[str, Any]]) -> dict[str, Any]:
    if block_number in cache:
        return cache[block_number]

    block_hex = hex(block_number)
    block = rpc("eth_getBlockByNumber", [block_hex, True])
    receipts = rpc("eth_getBlockReceipts", [block_hex])
    if not block:
        raise RuntimeError(f"block {block_number} not found")
    if receipts is None:
        raise RuntimeError(f"receipts for block {block_number} not found")

    receipt_by_hash = {str(item["transactionHash"]).lower(): item for item in receipts}
    base_fee = h2i(block.get("baseFeePerGas"))
    tx_rows = []
    priorities = []
    for tx in block.get("transactions", []):
        tx_hash = str(tx.get("hash", "")).lower()
        receipt = receipt_by_hash.get(tx_hash)
        if not receipt:
            continue
        gas_used = h2i(receipt.get("gasUsed"))
        effective_gas_price = h2i(receipt.get("effectiveGasPrice"))
        priority = max(0, effective_gas_price - base_fee)
        priorities.append(priority)
        tx_rows.append(
            {
                "tx_hash": tx_hash,
                "tx_index": h2i(tx.get("transactionIndex")),
                "gas_used": gas_used,
                "effective_gas_price_wei": effective_gas_price,
                "effective_priority_fee_wei": priority,
            }
        )

    priorities_desc = sorted(priorities, reverse=True)
    priorities_asc = sorted(priorities)
    gas_limit = h2i(block.get("gasLimit"))
    gas_used = h2i(block.get("gasUsed"))
    sample = {
        "block_number": block_number,
        "block_hash": block.get("hash"),
        "timestamp": h2i(block.get("timestamp")),
        "base_fee_wei": base_fee,
        "base_fee_gwei": wei_to_gwei(base_fee),
        "gas_limit": gas_limit,
        "gas_used": gas_used,
        "gas_used_ratio": Decimal(gas_used) / Decimal(gas_limit) if gas_limit else Decimal("0"),
        "tx_count": len(priorities),
        "priority_min_wei": priorities_asc[0] if priorities_asc else 0,
        "priority_p50_wei": quantile(priorities_asc, 0.50),
        "priority_p75_wei": quantile(priorities_asc, 0.75),
        "priority_p90_wei": quantile(priorities_asc, 0.90),
        "priority_p95_wei": quantile(priorities_asc, 0.95),
        "tail_tip_wei": (priorities_asc[0] + 1) if priorities_asc else 0,
        "top50_tip_wei": required_tip_for_position(priorities_desc, 50),
        "top25_tip_wei": required_tip_for_position(priorities_desc, 25),
        "top10_tip_wei": required_tip_for_position(priorities_desc, 10),
        "top5_tip_wei": required_tip_for_position(priorities_desc, 5),
        "rank_0gwei": rank_for_tip(priorities_desc, 0)[0],
        "rank_1gwei": rank_for_tip(priorities_desc, GWEI_1)[0],
        "rank_2gwei": rank_for_tip(priorities_desc, GWEI_2)[0],
        "rank_5gwei": rank_for_tip(priorities_desc, GWEI_5)[0],
        "priorities_desc": priorities_desc,
        "tx_rows": tx_rows,
    }
    cache[block_number] = sample
    return sample


def fetch_trade_details(trades: list[dict[str, Any]]) -> list[dict[str, Any]]:
    details = []
    for idx, trade in enumerate(trades, start=1):
        trade_id = trade["trade_id"]
        print(f"[{idx:03d}/{len(trades):03d}] fetch trade {trade_id}")
        details.append(
            api(
                "result-sets/"
                f"{RESULT_SET_ID}/strategies/{STRATEGY}/trades/{trade_id}"
            )
        )
    return details


def report_pairs(reports: list[dict[str, Any]]) -> list[tuple[dict[str, Any], dict[str, Any] | None]]:
    by_order: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for report in reports:
        by_order[str(report.get("order_id") or "")].append(report)

    pairs = []
    for order_id, rows in by_order.items():
        rows.sort(key=lambda row: (int(row.get("block_number") or 0), int(row.get("id") or 0)))
        submitted = [row for row in rows if str(row.get("event_type", "")).endswith("_submitted")]
        terminal = [
            row
            for row in rows
            if row.get("status") in {"confirmed", "failed", "cancelled"}
            and not str(row.get("event_type", "")).endswith("_submitted")
        ]
        for submit in submitted:
            term = next((row for row in terminal if row.get("order_id") == order_id), None)
            pairs.append((submit, term))
    return sorted(pairs, key=lambda pair: (int(pair[0].get("block_number") or 0), pair[0].get("order_id") or ""))


def gas_rows(details: list[dict[str, Any]]) -> tuple[list[dict[str, Any]], dict[str, dict[str, Decimal]]]:
    block_cache: dict[int, dict[str, Any]] = {}
    rows = []
    trade_priority_costs: dict[str, dict[str, Decimal]] = defaultdict(lambda: defaultdict(Decimal))

    all_pairs = []
    for detail in details:
        trade = detail.get("trade") or {}
        for submit, terminal in report_pairs(detail.get("reports") or []):
            all_pairs.append((detail, trade, submit, terminal))

    for idx, (detail, trade, submit, terminal) in enumerate(all_pairs, start=1):
        trade_id = trade.get("trade_id") or detail.get("trade_id")
        event_type = submit.get("event_type") or ""
        side = submit.get("order_side") or event_type.replace("_submitted", "")
        submit_block = int(submit.get("block_number") or 0)
        confirm_block = int(terminal.get("block_number") or submit_block + 1) if terminal else submit_block + 1
        print(
            f"[gas {idx:03d}/{len(all_pairs):03d}] {side} {trade_id} "
            f"submit {submit_block} confirm {confirm_block}"
        )
        sample = block_fee_sample(confirm_block, block_cache)
        gas_used = int((terminal or {}).get("gas_used") or 0)
        gas_cost_eth = dec((terminal or {}).get("gas_cost_eth"))
        implied_gas_price_wei = int((gas_cost_eth * WEI_PER_ETH / Decimal(gas_used)).to_integral_value()) if gas_used else 0
        implied_tip_wei = max(0, implied_gas_price_wei - int(sample["base_fee_wei"]))
        rank_recorded, recorded_equal = rank_for_tip(sample["priorities_desc"], implied_tip_wei)

        exact_costs = {
            "tail": priority_spend_eth(int(sample["tail_tip_wei"]), gas_used),
            "top50": priority_spend_eth(int(sample["top50_tip_wei"]), gas_used),
            "top25": priority_spend_eth(int(sample["top25_tip_wei"]), gas_used),
            "top10": priority_spend_eth(int(sample["top10_tip_wei"]), gas_used),
            "top5": priority_spend_eth(int(sample["top5_tip_wei"]), gas_used),
            "1gwei": priority_spend_eth(GWEI_1, gas_used),
            "2gwei": priority_spend_eth(GWEI_2, gas_used),
            "5gwei": priority_spend_eth(GWEI_5, gas_used),
        }
        for key, value in exact_costs.items():
            trade_priority_costs[trade_id][key] += value

        row = {
            "trade_id": trade_id,
            "token_address": detail.get("token_address") or trade.get("token_address"),
            "pool_address": detail.get("pool_address") or trade.get("pool_address"),
            "side": side,
            "submit_event": event_type,
            "submit_block": submit_block,
            "confirm_event": (terminal or {}).get("event_type"),
            "confirm_block": confirm_block,
            "delay_blocks": confirm_block - submit_block,
            "terminal_status": (terminal or {}).get("status"),
            "gas_used": gas_used,
            "recorded_gas_cost_eth": fmt_eth(gas_cost_eth, 12),
            "recorded_implied_gas_price_gwei": fmt_gwei(wei_to_gwei(implied_gas_price_wei), 6),
            "recorded_implied_priority_gwei": fmt_gwei(wei_to_gwei(implied_tip_wei), 6),
            "recorded_rank": rank_recorded,
            "recorded_equal_priority_ties": recorded_equal,
            "block_tx_count": sample["tx_count"],
            "block_gas_used_ratio": fmt_pct(sample["gas_used_ratio"] * Decimal("100"), 3),
            "block_base_fee_gwei": fmt_gwei(sample["base_fee_gwei"], 6),
            "block_priority_min_gwei": fmt_gwei(wei_to_gwei(sample["priority_min_wei"]), 6),
            "block_priority_p50_gwei": fmt_gwei(wei_to_gwei(sample["priority_p50_wei"]), 6),
            "block_priority_p75_gwei": fmt_gwei(wei_to_gwei(sample["priority_p75_wei"]), 6),
            "block_priority_p90_gwei": fmt_gwei(wei_to_gwei(sample["priority_p90_wei"]), 6),
            "block_priority_p95_gwei": fmt_gwei(wei_to_gwei(sample["priority_p95_wei"]), 6),
            "tail_tip_gwei": fmt_gwei(wei_to_gwei(sample["tail_tip_wei"]), 6),
            "top50_tip_gwei": fmt_gwei(wei_to_gwei(sample["top50_tip_wei"]), 6),
            "top25_tip_gwei": fmt_gwei(wei_to_gwei(sample["top25_tip_wei"]), 6),
            "top10_tip_gwei": fmt_gwei(wei_to_gwei(sample["top10_tip_wei"]), 6),
            "top5_tip_gwei": fmt_gwei(wei_to_gwei(sample["top5_tip_wei"]), 6),
            "rank_at_1gwei": sample["rank_1gwei"],
            "rank_at_2gwei": sample["rank_2gwei"],
            "rank_at_5gwei": sample["rank_5gwei"],
            "priority_spend_tail_eth": fmt_eth(exact_costs["tail"], 12),
            "priority_spend_top50_eth": fmt_eth(exact_costs["top50"], 12),
            "priority_spend_top25_eth": fmt_eth(exact_costs["top25"], 12),
            "priority_spend_top10_eth": fmt_eth(exact_costs["top10"], 12),
            "priority_spend_top5_eth": fmt_eth(exact_costs["top5"], 12),
            "priority_spend_1gwei_eth": fmt_eth(exact_costs["1gwei"], 12),
            "priority_spend_2gwei_eth": fmt_eth(exact_costs["2gwei"], 12),
            "priority_spend_5gwei_eth": fmt_eth(exact_costs["5gwei"], 12),
        }
        rows.append(row)
    return rows, trade_priority_costs


def snapshot_stats(snapshots: list[dict[str, Any]]) -> dict[str, Any]:
    if not snapshots:
        return {
            "snapshot_count": 0,
            "max_pnl_eth": Decimal("0"),
            "max_pnl_block": None,
            "min_pnl_eth": Decimal("0"),
            "min_pnl_block": None,
            "max_roi_percent": Decimal("0"),
            "max_roi_block": None,
            "min_roi_percent": Decimal("0"),
            "min_roi_block": None,
        }
    rows = []
    for snap in snapshots:
        rows.append(
            {
                "block": snap.get("block_number"),
                "pnl": dec(snap.get("total_pnl_eth")),
                "roi_percent": dec(snap.get("roi_percent")),
                "current_value_eth": dec(snap.get("current_value_eth")),
            }
        )
    max_pnl = max(rows, key=lambda row: row["pnl"])
    min_pnl = min(rows, key=lambda row: row["pnl"])
    max_roi = max(rows, key=lambda row: row["roi_percent"])
    min_roi = min(rows, key=lambda row: row["roi_percent"])
    return {
        "snapshot_count": len(rows),
        "max_pnl_eth": max_pnl["pnl"],
        "max_pnl_block": max_pnl["block"],
        "min_pnl_eth": min_pnl["pnl"],
        "min_pnl_block": min_pnl["block"],
        "max_roi_percent": max_roi["roi_percent"],
        "max_roi_block": max_roi["block"],
        "min_roi_percent": min_roi["roi_percent"],
        "min_roi_block": min_roi["block"],
    }


def improvement_note(trade: dict[str, Any], detail: dict[str, Any], stats: dict[str, Any]) -> str:
    state = str(trade.get("state") or "").lower()
    pnl = dec(trade.get("total_pnl_eth"))
    roi = dec(trade.get("roi_percent"))
    max_roi = dec(stats.get("max_roi_percent"))
    min_roi = dec(stats.get("min_roi_percent"))
    missed_peak = dec(stats.get("max_pnl_eth")) - pnl
    risks = detail.get("risks") or []
    risk_evidence = risk_evidence_summary(risks)
    sell_reason = (detail.get("token") or {}).get("sell_reason_label") or ""

    notes: list[str] = []
    if pnl >= Decimal("0.02"):
        notes.append("Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority.")
    elif pnl > Decimal("0.002"):
        notes.append("Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share.")
    elif pnl > 0:
        notes.append("Small winner; live execution should cap priority because the residual edge after bribe is thin.")
    elif pnl <= Decimal("-0.008"):
        notes.append("Major loss; this needs an avoid/fast-exit rule rather than just gas tuning.")
    else:
        notes.append("Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion.")

    if state != "sell_confirmed":
        if roi >= Decimal("50"):
            notes.append("Still open with material unrealized profit; live version should support take-profit or partial exit instead of waiting only for max hold.")
        elif roi <= Decimal("-80"):
            notes.append("Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value.")
        else:
            notes.append("Still open, so final quality is not known; deployment review should discount this mark until it exits.")

    if dec(stats.get("max_pnl_eth")) > Decimal("0.001") and missed_peak > Decimal("0.003") and max_roi - roi > Decimal("30"):
        notes.append(
            f"Peak mark was {fmt_eth(stats['max_pnl_eth'])} ETH at block {stats['max_pnl_block']}, "
            "so a trailing or take-profit exit would likely have improved this position."
        )
    elif dec(stats.get("max_pnl_eth")) < Decimal("0") and max_roi - roi > Decimal("20"):
        notes.append(
            "The least-bad mark was still negative and occurred earlier; a time-stop or stricter continuation filter would have reduced the current drawdown."
        )
    elif state == "sell_confirmed" and sell_reason and "max hold" in sell_reason.lower() and pnl > 0:
        notes.append("Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone.")

    if risk_evidence:
        notes.append(f"Risk evidence observed: {risk_evidence}.")
    if min_roi <= Decimal("-80") and pnl > Decimal("-0.005"):
        notes.append("The position had a deep adverse mark and then recovered; a hard stop should be paired with scam-specific evidence, not only mark-to-market drawdown.")

    return " ".join(notes)


def position_rows(
    details: list[dict[str, Any]],
    priority_costs: dict[str, dict[str, Decimal]],
) -> list[dict[str, Any]]:
    rows = []
    for detail in details:
        trade = detail.get("trade") or {}
        token = detail.get("token") or {}
        trade_id = trade.get("trade_id") or detail.get("trade_id")
        stats = snapshot_stats(detail.get("snapshots") or [])
        costs = priority_costs.get(trade_id, {})
        pnl = dec(trade.get("total_pnl_eth"))
        trigger_risk = sell_trigger_risk(detail)
        trigger_hash = str((trigger_risk or {}).get("pending_tx_hash") or "")
        risks = detail.get("risks") or []
        row = {
            "trade_id": trade_id,
            "state": trade.get("state"),
            "token_address": trade.get("token_address") or detail.get("token_address"),
            "pool_address": trade.get("pool_address") or detail.get("pool_address"),
            "entry_block": trade.get("entry_block"),
            "exit_block": trade.get("exit_block"),
            "entry_cost_eth": fmt_eth(trade.get("entry_cost_eth"), 6),
            "exit_value_eth": fmt_eth(trade.get("exit_value_eth"), 6) if trade.get("exit_value_eth") is not None else "",
            "current_value_eth": fmt_eth(trade.get("current_value_eth"), 6),
            "gas_cost_eth": fmt_eth(trade.get("gas_cost_eth"), 9),
            "realized_pnl_eth": fmt_eth(trade.get("realized_pnl_eth"), 6),
            "unrealized_pnl_eth": fmt_eth(trade.get("unrealized_pnl_eth"), 6),
            "total_pnl_eth": fmt_eth(pnl, 6),
            "roi_percent": fmt_pct(trade.get("roi_percent"), 2),
            "sell_reason": token.get("sell_reason_label") or "",
            "sell_reason_source": token.get("sell_reason_source") or "",
            "sell_reason_block": token.get("sell_reason_block") or "",
            "sell_trigger_kind": source_qualified_kind(trigger_risk) if trigger_risk else "",
            "sell_trigger_pending_tx_hash": trigger_hash,
            "risk_evidence": risk_evidence_summary(risks, include_pending_hash=False),
            "snapshot_count": stats["snapshot_count"],
            "max_pnl_eth": fmt_eth(stats["max_pnl_eth"], 6),
            "max_pnl_block": stats["max_pnl_block"] or "",
            "min_pnl_eth": fmt_eth(stats["min_pnl_eth"], 6),
            "min_pnl_block": stats["min_pnl_block"] or "",
            "max_roi_percent": fmt_pct(stats["max_roi_percent"], 2),
            "min_roi_percent": fmt_pct(stats["min_roi_percent"], 2),
            "priority_cost_tail_eth": fmt_eth(costs.get("tail", Decimal("0")), 9),
            "priority_cost_top50_eth": fmt_eth(costs.get("top50", Decimal("0")), 9),
            "priority_cost_top25_eth": fmt_eth(costs.get("top25", Decimal("0")), 9),
            "priority_cost_top10_eth": fmt_eth(costs.get("top10", Decimal("0")), 9),
            "priority_cost_2gwei_eth": fmt_eth(costs.get("2gwei", Decimal("0")), 9),
            "pnl_after_tail_priority_eth": fmt_eth(pnl - costs.get("tail", Decimal("0")), 6),
            "pnl_after_top25_priority_eth": fmt_eth(pnl - costs.get("top25", Decimal("0")), 6),
            "pnl_after_top10_priority_eth": fmt_eth(pnl - costs.get("top10", Decimal("0")), 6),
            "pnl_after_2gwei_priority_eth": fmt_eth(pnl - costs.get("2gwei", Decimal("0")), 6),
            "review": improvement_note(trade, detail, stats),
        }
        rows.append(row)
    rows.sort(key=lambda row: (int(row.get("entry_block") or 0), row["trade_id"]))
    return rows


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    if not rows:
        path.write_text("", encoding="utf-8")
        return
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)


def median_decimal(values: list[Decimal]) -> Decimal:
    if not values:
        return Decimal("0")
    return Decimal(str(statistics.median(values)))


def p90_decimal(values: list[Decimal]) -> Decimal:
    if not values:
        return Decimal("0")
    ordered = sorted(values)
    idx = round((len(ordered) - 1) * 0.90)
    return ordered[idx]


def summarize(
    summary_payload: dict[str, Any],
    positions: list[dict[str, Any]],
    gas: list[dict[str, Any]],
    validation_payload: dict[str, Any] | None,
    assessment_payload: dict[str, Any] | None,
) -> dict[str, Any]:
    result = summary_payload.get("result_set") or {}
    summary = summary_payload.get("summary") or {}
    gas_dec = lambda key: [dec(row.get(key)) for row in gas if row.get(key) not in (None, "")]
    pos_pnl = [dec(row["total_pnl_eth"]) for row in positions]
    priority_top25 = sum(gas_dec("priority_spend_top25_eth"), Decimal("0"))
    priority_top10 = sum(gas_dec("priority_spend_top10_eth"), Decimal("0"))
    priority_2gwei = sum(gas_dec("priority_spend_2gwei_eth"), Decimal("0"))
    priority_5gwei = sum(gas_dec("priority_spend_5gwei_eth"), Decimal("0"))
    total_pnl = dec(summary.get("total_pnl_eth")) if summary else sum(pos_pnl, Decimal("0"))
    assessment_questions = (assessment_payload or {}).get("questions") or []
    return {
        "generated_at_unix": int(time.time()),
        "asena_url": ASENA_URL,
        "eth_rpc_url": ETH_RPC_URL,
        "result_set_id": RESULT_SET_ID,
        "run_id": RUN_ID,
        "strategy": STRATEGY,
        "run_updated_at": result.get("updated_at"),
        "result_status": result.get("status"),
        "positions": len(positions),
        "closed_positions": sum(1 for row in positions if row["state"] == "sell_confirmed"),
        "open_positions": sum(1 for row in positions if row["state"] != "sell_confirmed"),
        "winners": sum(1 for value in pos_pnl if value > 0),
        "losers": sum(1 for value in pos_pnl if value < 0),
        "entry_cost_eth": summary.get("buy_volume_eth") or summary.get("entry_cost_eth"),
        "total_pnl_eth": fmt_eth(total_pnl, 9),
        "realized_pnl_eth": fmt_eth(summary.get("realized_pnl_eth"), 9),
        "unrealized_pnl_eth": fmt_eth(summary.get("unrealized_pnl_eth"), 9),
        "roi_percent": fmt_pct(summary.get("roi_percent"), 3),
        "submitted_orders": len(gas),
        "priority_spend_top25_eth": fmt_eth(priority_top25, 9),
        "priority_spend_top10_eth": fmt_eth(priority_top10, 9),
        "priority_spend_2gwei_eth": fmt_eth(priority_2gwei, 9),
        "priority_spend_5gwei_eth": fmt_eth(priority_5gwei, 9),
        "pnl_after_all_top25_priority_eth": fmt_eth(total_pnl - priority_top25, 9),
        "pnl_after_all_top10_priority_eth": fmt_eth(total_pnl - priority_top10, 9),
        "pnl_after_all_2gwei_priority_eth": fmt_eth(total_pnl - priority_2gwei, 9),
        "pnl_after_all_5gwei_priority_eth": fmt_eth(total_pnl - priority_5gwei, 9),
        "gas_tip_median_top25_gwei": fmt_gwei(median_decimal(gas_dec("top25_tip_gwei")), 6),
        "gas_tip_p90_top25_gwei": fmt_gwei(p90_decimal(gas_dec("top25_tip_gwei")), 6),
        "gas_tip_median_top10_gwei": fmt_gwei(median_decimal(gas_dec("top10_tip_gwei")), 6),
        "gas_tip_p90_top10_gwei": fmt_gwei(p90_decimal(gas_dec("top10_tip_gwei")), 6),
        "gas_rank_median_at_2gwei": median_decimal([dec(row["rank_at_2gwei"]) for row in gas]),
        "gas_rank_p90_at_2gwei": p90_decimal([dec(row["rank_at_2gwei"]) for row in gas]),
        "validation_status": (validation_payload or {}).get("status"),
        "validation_overall_verdict": (validation_payload or {}).get("overall_verdict"),
        "validation_passed": (validation_payload or {}).get("passed"),
        "validation_warnings": (validation_payload or {}).get("warnings"),
        "validation_failures": (validation_payload or {}).get("failures"),
        "validation_blocked": (validation_payload or {}).get("blocked"),
        "assessment_profit_factor": (assessment_payload or {}).get("summary", {}).get("profit_factor"),
        "assessment_top5_share_of_net_pnl_percent": (assessment_payload or {}).get("summary", {}).get("top5_share_of_net_pnl_percent"),
        "assessment_exposure_to_capital_percent": (assessment_payload or {}).get("summary", {}).get("exposure_to_capital_percent"),
        "assessment_questions": [
            {
                "code": question.get("code"),
                "assessment": question.get("assessment"),
                "answer": question.get("answer"),
            }
            for question in assessment_questions
        ],
    }


def write_summary_md(path: Path, summary: dict[str, Any]) -> None:
    text = f"""# Alpha11 Hold15 Deploy Review Summary

- result set: `{summary['result_set_id']}`
- strategy: `{summary['strategy']}`
- status: `{summary['result_status']}`
- run updated at: `{summary['run_updated_at']}`
- submitted orders reviewed: {summary['submitted_orders']}

## Current Performance

| metric | value |
| --- | ---: |
| positions | {summary['positions']} |
| closed | {summary['closed_positions']} |
| open | {summary['open_positions']} |
| winners | {summary['winners']} |
| losers | {summary['losers']} |
| total PnL ETH | {summary['total_pnl_eth']} |
| realized PnL ETH | {summary['realized_pnl_eth']} |
| unrealized PnL ETH | {summary['unrealized_pnl_eth']} |
| ROI | {summary['roi_percent']}% |

## Existing Lab Validation

| metric | value |
| --- | ---: |
| status | {summary.get('validation_status') or '-'} |
| overall verdict | {summary.get('validation_overall_verdict') or '-'} |
| passed checks | {summary.get('validation_passed') if summary.get('validation_passed') is not None else '-'} |
| warnings | {summary.get('validation_warnings') if summary.get('validation_warnings') is not None else '-'} |
| failures | {summary.get('validation_failures') if summary.get('validation_failures') is not None else '-'} |
| blocked | {summary.get('validation_blocked') if summary.get('validation_blocked') is not None else '-'} |

## Existing Lab Assessment

| metric | value |
| --- | ---: |
| profit factor | {summary.get('assessment_profit_factor') or '-'} |
| top5/net PnL | {fmt_pct(summary.get('assessment_top5_share_of_net_pnl_percent') or 0)}% |
| exposure/capital | {fmt_pct(summary.get('assessment_exposure_to_capital_percent') or 0)}% |

"""
    questions = summary.get("assessment_questions") or []
    if questions:
        text += "| question | assessment | answer |\n| --- | --- | --- |\n"
        for question in questions:
            text += (
                f"| {question.get('code') or '-'} | {question.get('assessment') or '-'} | "
                f"{question.get('answer') or '-'} |\n"
            )
        text += "\n"

    text += f"""

## Priority Fee Impact

The current chain-sim PnL includes simulated gas cost, but not the public
priority spend required to rank in the next mined block. The table below
subtracts only additional priority spend from current PnL.

| policy | added priority ETH | PnL after priority ETH |
| --- | ---: | ---: |
| exact next-block top 25 | {summary['priority_spend_top25_eth']} | {summary['pnl_after_all_top25_priority_eth']} |
| exact next-block top 10 | {summary['priority_spend_top10_eth']} | {summary['pnl_after_all_top10_priority_eth']} |
| fixed 2 gwei tip | {summary['priority_spend_2gwei_eth']} | {summary['pnl_after_all_2gwei_priority_eth']} |
| fixed 5 gwei tip | {summary['priority_spend_5gwei_eth']} | {summary['pnl_after_all_5gwei_priority_eth']} |

## Next-Block Rank Calibration

| metric | value |
| --- | ---: |
| median top-25 required tip | {summary['gas_tip_median_top25_gwei']} gwei |
| p90 top-25 required tip | {summary['gas_tip_p90_top25_gwei']} gwei |
| median top-10 required tip | {summary['gas_tip_median_top10_gwei']} gwei |
| p90 top-10 required tip | {summary['gas_tip_p90_top10_gwei']} gwei |
| median rank at 2 gwei | {summary['gas_rank_median_at_2gwei']} |
| p90 rank at 2 gwei | {summary['gas_rank_p90_at_2gwei']} |

## Initial Deployment Reading

- Use `gas_rank_review.csv` as the per-submit source of truth. It records the
  exact next mined block for each simulated submit and the priority fee needed
  for tail/top-50/top-25/top-10/top-5 placement.
- A fixed 2 gwei priority fee is a reasonable first baseline in this sample:
  it ranks near the front of most sampled confirmation blocks while adding less
  priority spend than 5 gwei.
- Top-10 placement should be reserved for exits where slippage or rug timing is
  more expensive than the extra priority. It is not free enough to use blindly
  on every buy.
- Current open exposure makes the headline PnL unstable. Treat open positions
  in `position_reviews.md` as incomplete until they close.
"""
    path.write_text(text, encoding="utf-8")


def sell_reason_line(row: dict[str, Any]) -> str:
    reason = row.get("sell_reason") or "-"
    source = row.get("sell_reason_source") or ""
    signal_block = row.get("sell_reason_block") or ""
    trigger = row.get("sell_trigger_kind") or ""
    pending_hash = row.get("sell_trigger_pending_tx_hash") or ""
    if not any([source, signal_block, trigger, pending_hash]):
        return f"- sell reason: `{reason}`"

    parts = [f"- sell reason: `{reason}`"]
    if trigger:
        parts.append(f"trigger: `{trigger}`")
    if source:
        parts.append(f"source: `{source}`")
    if signal_block:
        parts.append(f"signal block: `{signal_block}`")
    if pending_hash:
        parts.append(f"pending tx: `{short(str(pending_hash))}`")
    return "; ".join(parts)


def write_position_reviews(path: Path, rows: list[dict[str, Any]], gas_by_trade: dict[str, list[dict[str, Any]]]) -> None:
    lines = [
        "# Position Reviews",
        "",
        f"Result set: `{RESULT_SET_ID}`",
        f"Strategy: `{STRATEGY}`",
        "",
        "Each section states how the position performed, whether the backtest could have done better, and the exact next-block priority evidence for its submitted orders.",
        "",
    ]
    for idx, row in enumerate(rows, start=1):
        trade_id = row["trade_id"]
        lines.extend(
            [
                f"## {idx:02d}. `{trade_id}`",
                "",
                f"- token: `{row['token_address']}`",
                f"- pool: `{row['pool_address']}`",
                f"- state: `{row['state']}`; blocks: `{row['entry_block']}` -> `{row['exit_block'] or 'open'}`",
                f"- PnL: `{row['total_pnl_eth']}` ETH; ROI: `{row['roi_percent']}%`; current value: `{row['current_value_eth']}` ETH",
                f"- peak/trough: max `{row['max_pnl_eth']}` ETH at block `{row['max_pnl_block']}`, min `{row['min_pnl_eth']}` ETH at block `{row['min_pnl_block']}`",
                sell_reason_line(row),
                f"- after priority: top25 `{row['pnl_after_top25_priority_eth']}` ETH, top10 `{row['pnl_after_top10_priority_eth']}` ETH, fixed 2 gwei `{row['pnl_after_2gwei_priority_eth']}` ETH",
                f"- review: {row['review']}",
                "",
            ]
        )
        events = gas_by_trade.get(trade_id, [])
        if events:
            lines.append("| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |")
            lines.append("| --- | ---: | ---: | ---: | ---: | ---: | ---: |")
            for event in events:
                lines.append(
                    "| {side} | {submit_block}->{confirm_block} | {gas_used} | {top25_tip_gwei} gwei | {top10_tip_gwei} gwei | {rank_at_2gwei} | {priority_spend_2gwei_eth} ETH |".format(
                        **event
                    )
                )
            lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    print(f"Fetching result summary from {API_BASE}")
    summary_payload = api(f"result-sets/{RESULT_SET_ID}?strategy_name={STRATEGY}")
    validation_payload = api_optional(f"result-sets/{RESULT_SET_ID}/strategies/{STRATEGY}/validation")
    assessment_payload = api_optional(f"result-sets/{RESULT_SET_ID}/strategies/{STRATEGY}/assessment")
    trades_payload = api(f"result-sets/{RESULT_SET_ID}/strategies/{STRATEGY}/trades?limit=10000")
    trades = trades_payload.get("trades") or []
    details = fetch_trade_details(trades)
    gas, priority_costs = gas_rows(details)
    positions = position_rows(details, priority_costs)
    summary = summarize(summary_payload, positions, gas, validation_payload, assessment_payload)

    gas_by_trade: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in gas:
        gas_by_trade[row["trade_id"]].append(row)

    write_csv(OUT_DIR / "positions.csv", positions)
    write_csv(OUT_DIR / "gas_rank_review.csv", gas)
    write_position_reviews(OUT_DIR / "position_reviews.md", positions, gas_by_trade)
    write_summary_md(OUT_DIR / "summary.md", summary)
    (OUT_DIR / "summary.json").write_text(json.dumps(summary, indent=2, default=str) + "\n", encoding="utf-8")
    if validation_payload is not None:
        (OUT_DIR / "validation.json").write_text(
            json.dumps(validation_payload, indent=2, default=str) + "\n",
            encoding="utf-8",
        )
    if assessment_payload is not None:
        (OUT_DIR / "assessment.json").write_text(
            json.dumps(assessment_payload, indent=2, default=str) + "\n",
            encoding="utf-8",
        )
    print(f"Wrote {OUT_DIR}")


if __name__ == "__main__":
    main()
