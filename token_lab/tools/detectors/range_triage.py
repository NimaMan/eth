#!/usr/bin/env python3
"""Triage a completed token range run for investigation candidates.

This is intentionally a lab tool, not a production risk engine. It reads the
token server's in-memory range-run cache through the API and emits structured
candidate issues for the next investigation step.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from collections import defaultdict
from collections.abc import Mapping
from dataclasses import asdict, dataclass, field
from typing import Any, Sequence


from range_triage_format import format_json, format_jsonl, format_markdown, format_table
from range_triage_utils import (
    EXTREME_PRICE_RATIO,
    LP_APPROVAL_HIGH_PERCENT,
    LP_APPROVAL_MEDIUM_PERCENT,
    LP_HOLDER_CONCENTRATION_HIGH_PERCENT,
    LP_HOLDER_CONCENTRATION_MEDIUM_PERCENT,
    SEVERITY_RANK,
    TINY_SUPPLY_PERCENT,
    VERY_EXTREME_PRICE_RATIO,
    as_int,
    clean_json,
    collapse_message,
    compact_text,
    contract_analysis_metrics,
    contract_analysis_next_step,
    eligibility_label,
    finite_number,
    is_burn_address,
    is_eligible_liquidity,
    is_low_liquidity,
    is_meaningfully_liquid,
    looks_like_liquidity_drain,
    looks_like_timeout,
    meaningful_liquidity_threshold,
    metric_number,
    nested_get,
    normalize_address,
    opt_str,
    pool_metrics,
    sample_range,
    sample_values,
    url_quote,
)


DEFAULT_API_BASE = "http://127.0.0.1:8765"

class ApiError(RuntimeError):
    pass


class ApiClient:
    def __init__(self, base_url: str, timeout_secs: float = 30.0) -> None:
        self.base_url = base_url.rstrip("/")
        self.timeout_secs = timeout_secs

    def get(self, path: str) -> dict[str, Any]:
        url = self.base_url + "/" + path.lstrip("/")
        request = urllib.request.Request(url, headers={"accept": "application/json"})
        try:
            with urllib.request.urlopen(request, timeout=self.timeout_secs) as response:
                raw = response.read()
        except urllib.error.HTTPError as error:
            body = error.read().decode("utf-8", errors="replace")
            raise ApiError(f"GET {url} failed with HTTP {error.code}: {body}") from error
        except urllib.error.URLError as error:
            raise ApiError(f"GET {url} failed: {error}") from error

        try:
            return json.loads(raw.decode("utf-8"))
        except json.JSONDecodeError as error:
            preview = raw[:240].decode("utf-8", errors="replace")
            raise ApiError(f"GET {url} returned invalid JSON: {preview}") from error


@dataclass
class IssueCandidate:
    kind: str
    severity: str
    status: str = "new"
    token_address: str | None = None
    pool_address: str | None = None
    protocol: str | None = None
    symbol: str | None = None
    range_start: int | None = None
    range_end: int | None = None
    block: int | None = None
    tx_hash: str | None = None
    evidence: list[str] = field(default_factory=list)
    metrics: dict[str, Any] = field(default_factory=dict)
    suggested_next_step: str = ""

    def sort_key(self) -> tuple[int, str, str, str]:
        return (
            -SEVERITY_RANK.get(self.severity, 0),
            self.kind,
            self.symbol or "",
            self.pool_address or self.token_address or "",
        )

    def as_json(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class RangeRunSnapshot:
    api_base: str
    run_id: str
    progress: dict[str, Any]
    tokens: list[dict[str, Any]]
    pools: list[dict[str, Any]]
    errors: list[dict[str, Any]]

    @property
    def start_block(self) -> int | None:
        return as_int(self.progress.get("start_block"))

    @property
    def end_block(self) -> int | None:
        return as_int(self.progress.get("end_block"))

    @property
    def token_by_address(self) -> dict[str, dict[str, Any]]:
        return {
            normalize_address(token.get("contract_address")): token
            for token in self.tokens
            if token.get("contract_address")
        }

    @classmethod
    def load(
        cls,
        client: ApiClient,
        run_selector: str,
        stderr: Any = sys.stderr,
    ) -> "RangeRunSnapshot":
        run_id, progress = resolve_run(client, run_selector)
        if stderr:
            print(f"loading run {run_id} from {client.base_url}", file=stderr)

        tokens_response = client.get(f"/runs/{url_quote(run_id)}/tokens")
        pools_response = client.get(f"/runs/{url_quote(run_id)}/pools")
        errors_response = client.get(f"/runs/{url_quote(run_id)}/errors")

        return cls(
            api_base=client.base_url,
            run_id=run_id,
            progress=progress,
            tokens=list(tokens_response.get("tokens") or []),
            pools=list(pools_response.get("pools") or []),
            errors=list(errors_response.get("errors") or []),
        )


def resolve_run(client: ApiClient, run_selector: str) -> tuple[str, dict[str, Any]]:
    selector = run_selector.strip()
    if selector == "active":
        progress = client.get("/runs/active")
        run_id = progress.get("id")
        if not run_id:
            raise ApiError("/runs/active did not include an id")
        return str(run_id), progress

    if selector == "latest":
        runs_response = client.get("/runs")
        runs = list(runs_response.get("runs") or [])
        if not runs:
            raise ApiError("/runs returned no runs")
        runs.sort(
            key=lambda run: (
                as_int(run.get("updated_at_unix_secs")) or 0,
                as_int(run.get("started_at_unix_secs")) or 0,
                str(run.get("id") or ""),
            ),
            reverse=True,
        )
        run_id = str(runs[0].get("id") or "")
        if not run_id:
            raise ApiError("latest run did not include an id")
        return run_id, client.get(f"/runs/{url_quote(run_id)}/progress")

    progress = client.get(f"/runs/{url_quote(selector)}/progress")
    run_id = progress.get("id") or selector
    return str(run_id), progress


class IssueDetector:
    name = "issue"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        raise NotImplementedError


class ServerIndexerErrorDetector(IssueDetector):
    name = "server_indexer_errors"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
        for error in snapshot.errors:
            message = str(error.get("message") or "unknown error")
            groups[collapse_message(message)].append(error)

        candidates = []
        for message, errors in groups.items():
            count = len(errors)
            blocks = sorted(
                {
                    block
                    for block in (as_int(error.get("block_number")) for error in errors)
                    if block is not None
                }
            )
            tx_hashes = [
                str(error.get("tx_hash"))
                for error in errors
                if error.get("tx_hash")
            ][:5]
            severity = "high" if count >= 10 or looks_like_timeout(message) else "medium"
            candidates.append(
                issue(
                    snapshot,
                    kind="server_indexer.error_cluster",
                    severity=severity,
                    evidence=[
                        f"count={count}",
                        f"message={message}",
                        sample_range("blocks", blocks),
                        sample_values("txs", tx_hashes),
                    ],
                    metrics={
                        "count": count,
                        "sample_blocks": blocks[:10],
                        "sample_tx_hashes": tx_hashes,
                    },
                    suggested_next_step=(
                        "separate infrastructure/indexer failures from token behavior before "
                        "creating token-level investigations"
                    ),
                )
            )
        return candidates


class LifecycleConsistencyDetector(IssueDetector):
    name = "lifecycle_consistency"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        candidates = []
        for token in snapshot.tokens:
            has_pools = bool(token.get("has_pools")) or (as_int(token.get("pool_count")) or 0) > 0
            trading_enabled = bool(token.get("trading_enabled"))
            lifecycle = str(token.get("lifecycle_status") or "").upper()
            if not has_pools and (trading_enabled or "TRADING" in lifecycle):
                candidates.append(
                    issue(
                        snapshot,
                        kind="lifecycle.token_without_pool_has_trading_state",
                        severity="high",
                        token=token,
                        evidence=[
                            "has_pools=false",
                            f"pool_count={token.get('pool_count')}",
                            f"trading_enabled={trading_enabled}",
                            f"lifecycle_status={token.get('lifecycle_status')}",
                        ],
                        metrics={
                            "pool_count": token.get("pool_count"),
                            "trading_enabled": trading_enabled,
                            "lifecycle_status": token.get("lifecycle_status"),
                        },
                        suggested_next_step=(
                            "remove or correct token-level pool-derived state; trading viability "
                            "must come from pool simulation"
                        ),
                    )
                )
        return candidates


class SimulatorParityDetector(IssueDetector):
    name = "simulator_parity"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        candidates = []
        for pool in snapshot.pools:
            runtime_state = pool.get("runtime_state") or {}
            view_can_buy = bool(pool.get("can_buy"))
            view_can_sell = bool(pool.get("can_sell"))
            sim_can_buy = bool(runtime_state.get("can_buy", pool.get("can_buy")))
            sim_can_sell = bool(runtime_state.get("can_sell", pool.get("can_sell")))
            if not sim_can_buy or sim_can_sell:
                continue

            reason = opt_str(pool.get("last_trading_failure_reason"))
            failure_class = opt_str(pool.get("last_trading_failure_class"))
            observed_sell_volume = finite_number(runtime_state.get("token_volume_in")) or 0.0
            observed_sell_denom_out = finite_number(runtime_state.get("denom_volume_out")) or 0.0
            observed_sell = observed_sell_volume > 0.0 and observed_sell_denom_out > 0.0
            eligible = is_eligible_liquidity(pool)
            if observed_sell and not view_can_sell and eligible:
                severity = "critical"
            elif observed_sell and eligible:
                severity = "high"
            elif observed_sell or is_meaningfully_liquid(pool):
                severity = "medium"
            else:
                severity = "low"

            evidence = [
                f"simulator_can_buy={sim_can_buy}",
                f"simulator_can_sell={sim_can_sell}",
                f"view_can_buy={view_can_buy}",
                f"view_can_sell={view_can_sell}",
                f"liquidity={metric_number(pool.get('denom_reserve'))} {pool.get('currency') or ''}".strip(),
                f"eligibility={eligibility_label(pool)}",
            ]
            if failure_class:
                evidence.append(f"failure_class={failure_class}")
            if reason:
                evidence.append(f"failure_reason={compact_text(reason, 140)}")
            else:
                evidence.append("failure_reason=missing")
            if observed_sell:
                evidence.append("observed_sell_volume_present=true")

            kind = "simulator_parity.cannot_sell"
            if observed_sell:
                kind = "simulator_parity.observed_sell_overrides_sim_failure"
                if not view_can_sell:
                    kind = "simulator_parity.observed_sell_but_sim_cannot_sell"
            elif not failure_class and not reason:
                kind = "simulator_parity.missing_failure_reason"

            candidates.append(
                issue(
                    snapshot,
                    kind=kind,
                    severity=severity,
                    pool=pool,
                    evidence=evidence,
                    metrics=pool_metrics(pool)
                    | {
                        "last_trading_failure_class": failure_class,
                        "last_trading_failure_reason": reason,
                        "observed_token_sell_volume": observed_sell_volume,
                        "observed_denom_out_volume": observed_sell_denom_out,
                    },
                    suggested_next_step=(
                        "check observed buys/sells and replay the actual route before treating "
                        "this as a honeypot"
                    ),
                )
            )
        return candidates


class RouteMismatchDetector(IssueDetector):
    name = "route_mismatch"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        candidates = []
        for pool in snapshot.pools:
            if not (bool(pool.get("can_buy")) and not bool(pool.get("can_sell"))):
                runtime_state = pool.get("runtime_state") or {}
                if not (
                    bool(runtime_state.get("can_buy"))
                    and not bool(runtime_state.get("can_sell"))
                ):
                    continue

            runtime_state = pool.get("runtime_state") or {}
            observed_sell = (
                (finite_number(runtime_state.get("token_volume_in")) or 0.0) > 0.0
                and (finite_number(runtime_state.get("denom_volume_out")) or 0.0) > 0.0
            )
            failure_class = str(pool.get("last_trading_failure_class") or "")
            reason = str(pool.get("last_trading_failure_reason") or "")
            transfer_failed = (
                "transfer_from_failed" in failure_class.lower()
                or "TRANSFER_FROM_FAILED" in reason
            )
            if observed_sell and transfer_failed:
                candidates.append(
                    issue(
                        snapshot,
                        kind="route_mismatch.observed_sell_with_transfer_from_failed_sim",
                        severity="critical" if is_eligible_liquidity(pool) else "medium",
                        pool=pool,
                        evidence=[
                            "observed token sell volume exists",
                            "classic simulator failed with transfer-from failure",
                            f"protocol={pool.get('protocol')}",
                            f"eligibility={eligibility_label(pool)}",
                        ],
                        metrics=pool_metrics(pool)
                        | {
                            "observed_token_sell_volume": runtime_state.get("token_volume_in"),
                            "observed_denom_out_volume": runtime_state.get("denom_volume_out"),
                        },
                        suggested_next_step=(
                            "extract the mined sell route; classify whether this is Universal "
                            "Router, Permit2, aggregator, or whitelist behavior"
                        ),
                    )
                )
        return candidates


class AmountDustParityDetector(IssueDetector):
    name = "amount_dust_parity"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        candidates = []
        for pool in snapshot.pools:
            reason = str(pool.get("last_trading_failure_reason") or "")
            failure_class = str(pool.get("last_trading_failure_class") or "")
            insufficient_input = "INSUFFICIENT_INPUT_AMOUNT" in reason
            zero_output = "buy_received_zero_tokens" in failure_class.lower()
            tiny_supply = (finite_number(pool.get("pooled_token_supply_percent")) or 0.0) <= TINY_SUPPLY_PERCENT
            dust_liquidity = str(pool.get("liquidity_level") or "").lower() in {"dust", "drained"}

            if not (insufficient_input or zero_output or (bool(pool.get("can_buy")) and not bool(pool.get("can_sell")) and tiny_supply)):
                continue

            kind = "amount_dust_parity.suspicious_sell_input"
            if insufficient_input:
                kind = "amount_dust_parity.insufficient_input_amount"
            elif zero_output:
                kind = "amount_dust_parity.zero_buy_output"

            candidates.append(
                issue(
                    snapshot,
                    kind=kind,
                    severity="high" if insufficient_input or zero_output else "medium",
                    pool=pool,
                    evidence=[
                        f"failure_class={failure_class or 'missing'}",
                        f"failure_reason={compact_text(reason, 140) or 'missing'}",
                        f"supply_in_pool_percent={metric_number(pool.get('pooled_token_supply_percent'))}",
                        f"liquidity_level={pool.get('liquidity_level')}",
                        f"dust_liquidity={dust_liquidity}",
                    ],
                    metrics=pool_metrics(pool),
                    suggested_next_step=(
                        "verify simulated buy output and sell input amount; distinguish dust, "
                        "transfer-tax shrinkage, and amount extraction bugs"
                    ),
                )
            )
        return candidates


class PriceAndSupplyDetector(IssueDetector):
    name = "price_and_supply"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        candidates = []
        for pool in snapshot.pools:
            raw_ratio = finite_number(pool.get("raw_price_ratio_to_initial"))
            displayed_ratio = finite_number(pool.get("price_ratio_to_initial"))
            ratio = raw_ratio if raw_ratio is not None else displayed_ratio
            supply_status = str(pool.get("supply_ratio_status") or "")
            supply_label = opt_str(pool.get("supply_ratio_label"))
            supply_percent = finite_number(pool.get("pooled_token_supply_percent"))

            if ratio is not None and ratio >= EXTREME_PRICE_RATIO:
                severity = "critical" if ratio >= VERY_EXTREME_PRICE_RATIO else "high"
                if not is_eligible_liquidity(pool):
                    severity = "high" if is_meaningfully_liquid(pool) else "medium"
                if is_low_liquidity(pool) or (supply_percent is not None and supply_percent <= TINY_SUPPLY_PERCENT):
                    candidates.append(
                        issue(
                            snapshot,
                            kind="numerical.extreme_price_ratio_low_quality_liquidity",
                            severity=severity,
                            pool=pool,
                            evidence=[
                                f"raw_price_ratio_to_initial={metric_number(raw_ratio)}",
                                f"display_price_ratio_to_initial={metric_number(displayed_ratio)}",
                                f"liquidity={metric_number(pool.get('denom_reserve'))} {pool.get('currency') or ''}".strip(),
                                f"supply_in_pool_percent={metric_number(supply_percent)}",
                                f"eligibility={eligibility_label(pool)}",
                            ],
                            metrics=pool_metrics(pool),
                            suggested_next_step=(
                                "treat the ratio as suspect until reserves and supply share prove "
                                "the price is not dust-driven"
                            ),
                        )
                    )

            if supply_status and supply_status != "ok":
                severity = "critical" if supply_label else "high"
                if not is_eligible_liquidity(pool):
                    severity = "high" if is_meaningfully_liquid(pool) else "medium"
                candidates.append(
                    issue(
                        snapshot,
                        kind="supply.pool_reserve_exceeds_total_supply",
                        severity=severity,
                        pool=pool,
                        evidence=[
                            f"supply_ratio_status={supply_status}",
                            f"supply_ratio_label={supply_label or 'missing'}",
                            f"supply_in_pool_percent={metric_number(supply_percent)}",
                            f"eligibility={eligibility_label(pool)}",
                        ],
                        metrics=pool_metrics(pool),
                        suggested_next_step=(
                            "check hidden mint, decimals, or reserve parsing before using FDV or "
                            "pool supply ratios"
                        ),
                    )
                )
            elif supply_percent is not None and supply_percent > 100.0001:
                candidates.append(
                    issue(
                        snapshot,
                        kind="supply.pool_supply_share_over_100_percent",
                        severity="critical" if is_eligible_liquidity(pool) else "high",
                        pool=pool,
                        evidence=[
                            f"supply_in_pool_percent={metric_number(supply_percent)}",
                            f"eligibility={eligibility_label(pool)}",
                        ],
                        metrics=pool_metrics(pool),
                        suggested_next_step=(
                            "verify total supply, token decimals, and reserve extraction"
                        ),
                    )
                )
        return candidates


class LiquidityHealthDetector(IssueDetector):
    name = "liquidity_health"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        candidates = []
        for pool in snapshot.pools:
            level = str(pool.get("liquidity_level") or "").lower()
            history = list(pool.get("liquidity_history") or [])
            current_liquidity = finite_number(pool.get("denom_reserve")) or 0.0
            max_liquidity = max(
                [finite_number(point.get("denom_reserve")) or 0.0 for point in history] + [current_liquidity]
            )

            has_trading_flag = (
                bool(pool.get("can_buy"))
                or bool(pool.get("can_sell"))
                or bool(pool.get("trading_enabled"))
            )
            if level in {"dust", "drained"} and has_trading_flag:
                candidates.append(
                    issue(
                        snapshot,
                        kind=f"liquidity.{level}_pool_marked_tradable",
                        severity="high" if level == "drained" else "medium",
                        pool=pool,
                        evidence=[
                            f"liquidity_level={level}",
                            f"can_buy={pool.get('can_buy')}",
                            f"can_sell={pool.get('can_sell')}",
                            f"trading_enabled={pool.get('trading_enabled')}",
                            f"current_liquidity={metric_number(current_liquidity)} {pool.get('currency') or ''}".strip(),
                        ],
                        metrics=pool_metrics(pool) | {"max_liquidity": max_liquidity},
                        suggested_next_step=(
                            "separate drained/dust pools from active pools and verify whether "
                            "trading viability is still meaningful"
                        ),
                    )
                )

            if looks_like_liquidity_drain(pool, current_liquidity, max_liquidity):
                candidates.append(
                    issue(
                        snapshot,
                        kind="liquidity.possible_drain",
                        severity="high",
                        pool=pool,
                        evidence=[
                            f"max_liquidity={metric_number(max_liquidity)} {pool.get('currency') or ''}".strip(),
                            f"current_liquidity={metric_number(current_liquidity)} {pool.get('currency') or ''}".strip(),
                            f"liquidity_level={pool.get('liquidity_level')}",
                        ],
                        metrics=pool_metrics(pool) | {"max_liquidity": max_liquidity},
                        suggested_next_step=(
                            "inspect reserve history and burn/sync events to confirm whether "
                            "liquidity was drained"
                        ),
                    )
                )

            lp_supply = finite_number(pool.get("lp_total_supply")) or 0.0
            if lp_supply <= 0.0 and current_liquidity > meaningful_liquidity_threshold(pool):
                lp_supply_known = bool(pool.get("lp_supply_known"))
                kind = "lp.zero_supply_with_reserves"
                severity = "high" if is_eligible_liquidity(pool) else "medium"
                if not lp_supply_known:
                    kind = "lp.unknown_supply_with_reserves"
                    severity = "medium" if is_eligible_liquidity(pool) else "low"
                candidates.append(
                    issue(
                        snapshot,
                        kind=kind,
                        severity=severity,
                        pool=pool,
                        evidence=[
                            f"lp_total_supply={metric_number(lp_supply)}",
                            f"lp_supply_status={pool.get('lp_supply_status') or 'unknown'}",
                            f"current_liquidity={metric_number(current_liquidity)} {pool.get('currency') or ''}".strip(),
                            f"eligibility={eligibility_label(pool)}",
                        ],
                        metrics=pool_metrics(pool) | {"lp_total_supply": lp_supply},
                        suggested_next_step=(
                            "verify LP supply accounting and whether protocol-specific liquidity "
                            "representation is being treated as V2 LP tokens"
                        ),
                    )
                )
        return candidates


class LpControlDetector(IssueDetector):
    name = "lp_control"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        candidates = []
        for pool in snapshot.pools:
            approved = finite_number(pool.get("lp_approved_percentage"))
            if approved is not None and approved >= LP_APPROVAL_MEDIUM_PERCENT:
                candidates.append(
                    issue(
                        snapshot,
                        kind="lp.approved_liquidity_exposure",
                        severity="high" if approved >= LP_APPROVAL_HIGH_PERCENT else "medium",
                        pool=pool,
                        evidence=[
                            f"lp_approved_percentage={metric_number(approved)}",
                            f"lp_approval_count={pool.get('lp_approval_count')}",
                            f"lp_holders_with_approvals={len(pool.get('lp_holders_with_approvals') or [])}",
                        ],
                        metrics=pool_metrics(pool)
                        | {
                            "lp_approved_percentage": approved,
                            "lp_approval_count": pool.get("lp_approval_count"),
                            "lp_holders_with_approvals": pool.get("lp_holders_with_approvals"),
                        },
                        suggested_next_step=(
                            "inspect LP approval owners and spenders; high approved LP can allow "
                            "rapid liquidity removal"
                        ),
                    )
                )

            for holder in pool.get("lp_holders") or []:
                share = finite_number(holder.get("share"))
                address = normalize_address(holder.get("address"))
                if share is None or is_burn_address(address):
                    continue
                if share > 100.0001:
                    candidates.append(
                        issue(
                            snapshot,
                            kind="lp.holder_share_over_100_percent",
                            severity="critical",
                            pool=pool,
                            evidence=[
                                f"holder={address}",
                                f"lp_share={metric_number(share)}",
                            ],
                            metrics=pool_metrics(pool) | {"holder": address, "lp_share": share},
                            suggested_next_step=(
                                "verify LP total supply and holder balance accounting"
                            ),
                        )
                    )
                elif share >= LP_HOLDER_CONCENTRATION_MEDIUM_PERCENT:
                    candidates.append(
                        issue(
                            snapshot,
                            kind="lp.concentrated_holder",
                            severity=(
                                "high"
                                if share >= LP_HOLDER_CONCENTRATION_HIGH_PERCENT
                                else "medium"
                            ),
                            pool=pool,
                            evidence=[
                                f"holder={address}",
                                f"lp_share={metric_number(share)}",
                                f"approvals={len(holder.get('approvals') or {})}",
                            ],
                            metrics=pool_metrics(pool)
                            | {
                                "holder": address,
                                "lp_share": share,
                                "approvals": holder.get("approvals") or {},
                            },
                            suggested_next_step=(
                                "check whether the holder is deployer/owner/locker and whether "
                                "the LP can be moved or approved"
                            ),
                        )
                    )
        return candidates


class TaxSafetyDetector(IssueDetector):
    name = "tax_safety"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        candidates = []
        for pool in snapshot.pools:
            bucket = str(pool.get("tax_bucket") or "unknown").lower()
            if bucket not in {"moderate_tax", "high_tax", "extreme_tax"}:
                continue
            severity = {
                "moderate_tax": "medium",
                "high_tax": "high",
                "extreme_tax": "critical",
            }[bucket]
            candidates.append(
                issue(
                    snapshot,
                    kind=f"tax.{bucket}",
                    severity=severity,
                    pool=pool,
                    evidence=[
                        f"buy_tax={metric_number(pool.get('buy_tax'))}",
                        f"sell_tax={metric_number(pool.get('sell_tax'))}",
                        f"buy_tax_bucket={pool.get('buy_tax_bucket')}",
                        f"sell_tax_bucket={pool.get('sell_tax_bucket')}",
                    ],
                    metrics=pool_metrics(pool),
                    suggested_next_step=(
                        "verify tax through simulator parity and decide whether this should be a "
                        "trading guardrail"
                    ),
                )
            )
        return candidates


class ProtocolCoverageDetector(IssueDetector):
    name = "protocol_coverage"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        candidates = []
        for pool in snapshot.pools:
            protocol = str(pool.get("protocol") or "").upper()
            if protocol in {"", "UNISWAP-V2"}:
                continue

            missing = []
            if protocol in {"UNISWAP-V3", "UNISWAP-V4"}:
                for field_name in ("current_tick", "sqrt_price_x96", "active_liquidity", "virtual_reserves"):
                    if pool.get(field_name) in (None, "", []):
                        missing.append(field_name)
            if protocol == "UNISWAP-V4":
                for field_name in ("pool_id", "pool_manager_address", "hooks"):
                    if pool.get(field_name) in (None, "", []):
                        missing.append(field_name)

            if missing:
                candidates.append(
                    issue(
                        snapshot,
                        kind="protocol_coverage.missing_protocol_metrics",
                        severity="medium",
                        pool=pool,
                        evidence=[
                            f"protocol={protocol}",
                            f"missing={','.join(missing)}",
                        ],
                        metrics=pool_metrics(pool) | {"missing_fields": missing},
                        suggested_next_step=(
                            "verify whether this is expected for the protocol or a missing "
                            "indexer/view field before comparing it to V2 pools"
                        ),
                    )
                )
        return candidates


class ContractAnalysisDetector(IssueDetector):
    name = "contract_analysis"

    def detect(self, snapshot: RangeRunSnapshot) -> list[IssueCandidate]:
        candidates = []
        for token in snapshot.tokens:
            analysis = token.get("contract_analysis") or {}
            if not analysis:
                continue

            for evidence in analysis.get("evidence") or []:
                severity = str(evidence.get("severity") or "info").lower()
                if SEVERITY_RANK.get(severity, 0) < SEVERITY_RANK["medium"]:
                    continue

                kind = str(evidence.get("kind") or "unknown")
                source = str(evidence.get("source") or "unknown")
                message = compact_text(str(evidence.get("message") or ""), 160)
                candidates.append(
                    issue(
                        snapshot,
                        kind=f"contract_analysis.{kind}",
                        severity=severity,
                        token=token,
                        evidence=[
                            f"source={source}",
                            f"message={message}",
                            f"interface_quality={nested_get(analysis, 'interface', 'quality')}",
                        ],
                        metrics=contract_analysis_metrics(analysis),
                        suggested_next_step=contract_analysis_next_step(kind),
                    )
                )
        return candidates


ALL_DETECTORS: list[IssueDetector] = [
    ServerIndexerErrorDetector(),
    LifecycleConsistencyDetector(),
    SimulatorParityDetector(),
    RouteMismatchDetector(),
    AmountDustParityDetector(),
    PriceAndSupplyDetector(),
    LiquidityHealthDetector(),
    LpControlDetector(),
    TaxSafetyDetector(),
    ProtocolCoverageDetector(),
    ContractAnalysisDetector(),
]


class RangeTriageRunner:
    def __init__(self, detectors: Sequence[IssueDetector] = ALL_DETECTORS) -> None:
        self.detectors = list(detectors)

    def run(
        self,
        snapshot: RangeRunSnapshot,
        detector_names: set[str] | None = None,
    ) -> list[IssueCandidate]:
        candidates: list[IssueCandidate] = []
        for detector in self.detectors:
            if detector_names and detector.name not in detector_names:
                continue
            candidates.extend(detector.detect(snapshot))
        candidates.sort(key=lambda candidate: candidate.sort_key())
        return candidates


def issue(
    snapshot: RangeRunSnapshot,
    *,
    kind: str,
    severity: str,
    token: Mapping[str, Any] | None = None,
    pool: Mapping[str, Any] | None = None,
    evidence: list[str],
    metrics: dict[str, Any],
    suggested_next_step: str,
) -> IssueCandidate:
    token = token or {}
    pool = pool or {}
    token_address = opt_str(pool.get("token_address")) or opt_str(token.get("contract_address"))
    return IssueCandidate(
        kind=kind,
        severity=severity,
        token_address=token_address,
        pool_address=opt_str(pool.get("pool_address")),
        protocol=opt_str(pool.get("protocol")),
        symbol=opt_str(pool.get("token_symbol")) or opt_str(token.get("symbol")),
        range_start=snapshot.start_block,
        range_end=snapshot.end_block,
        block=as_int(pool.get("latest_block_number"))
        or as_int(pool.get("creation_block"))
        or as_int(token.get("latest_block"))
        or as_int(token.get("creation_block")),
        tx_hash=None,
        evidence=[value for value in evidence if value and not value.endswith("=None")],
        metrics=clean_json(metrics),
        suggested_next_step=suggested_next_step,
    )


def run_detectors(
    snapshot: RangeRunSnapshot,
    detector_names: set[str] | None = None,
) -> list[IssueCandidate]:
    return RangeTriageRunner().run(snapshot, detector_names)


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Find token-lab investigation candidates in an existing range run.",
    )
    parser.add_argument(
        "--api",
        default=os.environ.get("ETH_TOKEN_API", DEFAULT_API_BASE),
        help=f"token server API base URL (default: {DEFAULT_API_BASE})",
    )
    parser.add_argument(
        "--run",
        default="active",
        help="run id to inspect, or 'active', or 'latest' (default: active)",
    )
    parser.add_argument(
        "--format",
        choices=("json", "jsonl", "markdown", "table"),
        default="table",
        help="output format (default: table)",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=200,
        help="maximum candidates to print; 0 means no limit (default: 200)",
    )
    parser.add_argument(
        "--min-severity",
        choices=("info", "low", "medium", "high", "critical"),
        default="low",
        help="minimum severity to print (default: low)",
    )
    parser.add_argument(
        "--detector",
        action="append",
        choices=[detector.name for detector in ALL_DETECTORS],
        help="run only this detector; can be repeated",
    )
    parser.add_argument(
        "--quiet",
        action="store_true",
        help="do not print loading progress to stderr",
    )
    return parser.parse_args(argv)


def filter_candidates(
    candidates: list[IssueCandidate],
    min_severity: str,
    limit: int,
) -> list[IssueCandidate]:
    min_rank = SEVERITY_RANK[min_severity]
    filtered = [
        candidate
        for candidate in candidates
        if SEVERITY_RANK.get(candidate.severity, 0) >= min_rank
    ]
    if limit > 0:
        return filtered[:limit]
    return filtered


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    try:
        client = ApiClient(args.api)
        snapshot = RangeRunSnapshot.load(
            client,
            args.run,
            stderr=None if args.quiet else sys.stderr,
        )
        candidates = run_detectors(snapshot, set(args.detector or []) or None)
        candidates = filter_candidates(candidates, args.min_severity, args.limit)

        if args.format == "json":
            output = format_json(snapshot, candidates)
        elif args.format == "jsonl":
            output = format_jsonl(candidates)
        elif args.format == "markdown":
            output = format_markdown(snapshot, candidates)
        else:
            output = format_table(candidates)

        print(output)
        return 0
    except ApiError as error:
        print(f"range_triage: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
