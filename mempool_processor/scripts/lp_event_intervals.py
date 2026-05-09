from __future__ import annotations

from collections import defaultdict
from datetime import datetime, timedelta
from pathlib import Path
import csv
import re
from typing import Iterable, Mapping


_TS_FORMAT = "%Y-%m-%d %H:%M:%S.%f"

_FN_LP_RE = re.compile(
    r"\[(?P<ts>[^\]]+)\]\sLP APPROVAL TX:\s(?P<tx>0x[0-9a-fA-F]+)\s\|\sFrom:\s(?P<creator>0x[0-9a-fA-F]+)"
    r"(?:\s\|\sLP Pair:\s(?P<lp_pair>0x[0-9a-fA-F]+))?",
)
_FN_REMOVAL_RE = re.compile(
    r"\[(?P<ts>[^\]]+)\]\sTX:\s(?P<tx>0x[0-9a-fA-F]+)\s\|\sFrom:\s(?P<creator>0x[0-9a-fA-F]+)"
    r"(?:\s\|\sTo:\s(?P<to>[^|]+?))?\s\|\sValue:\s(?P<value>[^|]+)\s\|\sGasPrice:\s(?P<gas_price>[^|]+)\s\|\sFunction:\s(?P<function>[^|]+)",
)

_SIG_LP_RE = re.compile(
    r"\[(?P<ts>[^\]]+)\]\sLP_APPROVAL\s\|\sCreator:\s(?P<creator>0x[0-9a-fA-F]+)\s\|\sLP Token:\s(?P<lp_token>0x[0-9a-fA-F]+)"
    r"\s\|\sRouter:\s(?P<router>0x[0-9a-fA-F]+)\s\|\sAmount:\s(?P<amount>[^|]+)\s\|\sTxHash:\s(?P<tx>0x[0-9a-fA-F]+)",
)
_SIG_REMOVAL_RE = re.compile(
    r"\[(?P<ts>[^\]]+)\]\s(?P<status>(?:LIQUIDITY_)?REMOVAL[^|]*)\s\|\sPool:\s(?P<pool>[^|]+)"
    r"\s\|\sToken:\s(?P<token>0x[0-9a-fA-F]+)\s\|\sRemover:\s(?P<creator>0x[0-9a-fA-F]+)"
    r"\s\|\sReason:\s(?P<reason>[^|]+)\s\|\sTxHash:\s(?P<tx>0x[0-9a-fA-F]+)",
    re.IGNORECASE,
)


def _parse_ts(ts: str) -> datetime:
    return datetime.strptime(ts, _TS_FORMAT)


def _collect_matches(path: Path, pattern: re.Pattern) -> list[dict[str, str]]:
    if not path.exists():
        return []
    matches: list[dict[str, str]] = []
    with path.open() as handle:
        for line in handle:
            match = pattern.search(line)
            if match:
                matches.append(match.groupdict())
    return matches


def _prepare_event(entries: Iterable[Mapping[str, str]], ts_field: str = "ts") -> list[dict[str, object]]:
    result: list[dict[str, object]] = []
    for entry in entries:
        struct = dict(entry)
        struct[ts_field] = _parse_ts(struct[ts_field])
        result.append(struct)
    return result


def _first_after(events: list[dict], ts_key: str, cutoff: datetime) -> dict | None:
    candidates = [evt for evt in events if evt[ts_key] >= cutoff]
    if not candidates:
        return None
    return min(candidates, key=lambda item: item[ts_key])


def _serialize_ts(dt: datetime | None) -> str:
    return dt.isoformat(timespec="microseconds") if dt else ""


def _serialize_float(value: float | None) -> str:
    return f"{value:.3f}" if value is not None else ""


def _estimate_removal_offset(buckets: Mapping[str, dict]) -> timedelta:
    diffs: list[float] = []
    for data in buckets.values():
        fn_events = data.get("fn_removal", [])
        if not fn_events:
            continue
        fn_by_tx = {evt.get("tx", ""): evt for evt in fn_events}
        for sig_evt in data.get("sig_removal", []):
            fn_evt = fn_by_tx.get(sig_evt.get("tx", ""))
            if fn_evt:
                diffs.append((sig_evt["ts"] - fn_evt["ts"]).total_seconds())
                if len(diffs) >= 5:  # enough samples
                    break
        if len(diffs) >= 5:
            break
    if not diffs:
        return timedelta(0)
    diffs.sort()
    median = diffs[len(diffs) // 2]
    return timedelta(seconds=median)


def compute_lp_event_intervals(run_dir: str | Path) -> list[dict[str, object]]:
    run_dir = Path(run_dir)
    buckets = defaultdict(lambda: {"fn_lp": [], "fn_removal": [], "sig_lp": [], "sig_removal": []})

    fn_log = run_dir / "function_detector" / "liquidity_removals.log"
    for entry in _prepare_event(_collect_matches(fn_log, _FN_LP_RE)):
        buckets[entry["creator"].lower()]["fn_lp"].append(entry)
    for entry in _prepare_event(_collect_matches(fn_log, _FN_REMOVAL_RE)):
        buckets[entry["creator"].lower()]["fn_removal"].append(entry)

    sig_lp_log = run_dir / "signals" / "lp_approval_signals.log"
    for entry in _prepare_event(_collect_matches(sig_lp_log, _SIG_LP_RE)):
        buckets[entry["creator"].lower()]["sig_lp"].append(entry)

    sig_removal_log = run_dir / "signals" / "liquidity_removals.log"
    for entry in _prepare_event(_collect_matches(sig_removal_log, _SIG_REMOVAL_RE)):
        buckets[entry["creator"].lower()]["sig_removal"].append(entry)

    removal_offset = _estimate_removal_offset(buckets)
    if removal_offset:
        for data in buckets.values():
            for event in data["sig_removal"]:
                event["ts"] = event["ts"] - removal_offset

    results: list[dict[str, object]] = []

    for creator, data in buckets.items():
        fn_lp_events = sorted(data["fn_lp"], key=lambda e: e["ts"])
        if not fn_lp_events:
            continue

        first_fn_lp = fn_lp_events[0]
        fn_removal_events = sorted(data["fn_removal"], key=lambda e: e["ts"])
        sig_lp_events = sorted(data["sig_lp"], key=lambda e: e["ts"])
        sig_removal_events = sorted(data["sig_removal"], key=lambda e: e["ts"])

        first_fn_removal = _first_after(fn_removal_events, "ts", first_fn_lp["ts"])
        first_sig_lp = sig_lp_events[0] if sig_lp_events else None
        first_sig_removal = _first_after(sig_removal_events, "ts", first_fn_lp["ts"]) if sig_removal_events else None

        delta_fn_lp_to_fn_removal = (
            (first_fn_removal["ts"] - first_fn_lp["ts"]).total_seconds()
            if first_fn_removal
            else None
        )
        delta_fn_lp_to_sig_removal = (
            (first_sig_removal["ts"] - first_fn_lp["ts"]).total_seconds()
            if first_sig_removal
            else None
        )
        delta_sig_lp_to_sig_removal = (
            (first_sig_removal["ts"] - first_sig_lp["ts"]).total_seconds()
            if first_sig_lp and first_sig_removal
            else None
        )
        delta_sig_lp_to_fn_removal = (
            (first_fn_removal["ts"] - first_sig_lp["ts"]).total_seconds()
            if first_sig_lp and first_fn_removal
            else None
        )

        results.append(
            {
                "creator": creator,
                "fn_lp": first_fn_lp,
                "fn_removal": first_fn_removal,
                "sig_lp": first_sig_lp,
                "sig_removal": first_sig_removal,
                "delta_fn_lp_to_fn_removal": delta_fn_lp_to_fn_removal,
                "delta_fn_lp_to_sig_removal": delta_fn_lp_to_sig_removal,
                "delta_sig_lp_to_sig_removal": delta_sig_lp_to_sig_removal,
                "delta_sig_lp_to_fn_removal": delta_sig_lp_to_fn_removal,
            }
        )

    results.sort(key=lambda row: row["fn_lp"]["ts"])
    return results


def export_lp_event_intervals_csv(run_dir: str | Path, output_path: Path | None = None) -> Path:
    records = compute_lp_event_intervals(run_dir)

    if output_path is None:
        base_dir = Path(__file__).resolve().parent
        output_path = base_dir / "lp_event_intervals.csv"

    columns = [
        "creator",
        "fn_lp_timestamp",
        "fn_lp_tx",
        "fn_lp_lp_pair",
        "sig_lp_timestamp",
        "sig_lp_tx",
        "sig_lp_token",
        "sig_lp_router",
        "sig_lp_amount",
        "fn_removal_timestamp",
        "fn_removal_tx",
        "fn_removal_function",
        "fn_removal_to",
        "sig_removal_timestamp",
        "sig_removal_tx",
        "sig_removal_status",
        "sig_removal_token",
        "sig_removal_pool",
        "sig_removal_reason",
        "delta_fn_lp_to_fn_removal_s",
        "delta_fn_lp_to_sig_removal_s",
        "delta_sig_lp_to_sig_removal_s",
        "delta_sig_lp_to_fn_removal_s",
    ]

    def _value(row: dict, section: str, key: str) -> str:
        event = row.get(section)
        if not event:
            return ""
        return _serialize_ts(event.get("ts")) if key == "ts" else str(event.get(key, ""))

    with output_path.open("w", newline="") as csvfile:
        writer = csv.DictWriter(csvfile, fieldnames=columns)
        writer.writeheader()
        for row in records:
            writer.writerow(
                {
                    "creator": row["creator"],
                    "fn_lp_timestamp": _value(row, "fn_lp", "ts"),
                    "fn_lp_tx": _value(row, "fn_lp", "tx"),
                    "fn_lp_lp_pair": _value(row, "fn_lp", "lp_pair"),
                    "sig_lp_timestamp": _value(row, "sig_lp", "ts"),
                    "sig_lp_tx": _value(row, "sig_lp", "tx"),
                    "sig_lp_token": _value(row, "sig_lp", "lp_token"),
                    "sig_lp_router": _value(row, "sig_lp", "router"),
                    "sig_lp_amount": _value(row, "sig_lp", "amount"),
                    "fn_removal_timestamp": _value(row, "fn_removal", "ts"),
                    "fn_removal_tx": _value(row, "fn_removal", "tx"),
                    "fn_removal_function": _value(row, "fn_removal", "function"),
                    "fn_removal_to": _value(row, "fn_removal", "to"),
                    "sig_removal_timestamp": _value(row, "sig_removal", "ts"),
                    "sig_removal_tx": _value(row, "sig_removal", "tx"),
                    "sig_removal_status": _value(row, "sig_removal", "status"),
                    "sig_removal_token": _value(row, "sig_removal", "token"),
                    "sig_removal_pool": _value(row, "sig_removal", "pool"),
                    "sig_removal_reason": _value(row, "sig_removal", "reason"),
                    "delta_fn_lp_to_fn_removal_s": _serialize_float(row["delta_fn_lp_to_fn_removal"]),
                    "delta_fn_lp_to_sig_removal_s": _serialize_float(row["delta_fn_lp_to_sig_removal"]),
                    "delta_sig_lp_to_sig_removal_s": _serialize_float(row["delta_sig_lp_to_sig_removal"]),
                    "delta_sig_lp_to_fn_removal_s": _serialize_float(row["delta_sig_lp_to_fn_removal"]),
                }
            )

    return output_path


if __name__ == "__main__":
    run_dir = Path(__file__).resolve().parents[1] / "logs" / "signal_detector_2025-09-14_11-00-25"
    csv_path = export_lp_event_intervals_csv(run_dir)
    print(f"Wrote {csv_path}")
