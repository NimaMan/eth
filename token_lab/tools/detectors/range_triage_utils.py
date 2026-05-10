from __future__ import annotations

import math
import urllib.parse
from collections.abc import Mapping, Sequence
from typing import Any


SEVERITY_RANK = {
    "critical": 5,
    "high": 4,
    "medium": 3,
    "low": 2,
    "info": 1,
}

WETH_LIQUIDITY_LOW = 1.0
WETH_LIQUIDITY_DUST = 0.01
WETH_LIQUIDITY_ELIGIBLE = 0.5
STABLE_LIQUIDITY_LOW = 1_000.0
STABLE_LIQUIDITY_DUST = 10.0
STABLE_LIQUIDITY_ELIGIBLE = 1_000.0
EXTREME_PRICE_RATIO = 1_000.0
VERY_EXTREME_PRICE_RATIO = 100_000.0
TINY_SUPPLY_PERCENT = 0.01
LP_APPROVAL_HIGH_PERCENT = 20.0
LP_APPROVAL_MEDIUM_PERCENT = 5.0
LP_HOLDER_CONCENTRATION_HIGH_PERCENT = 90.0
LP_HOLDER_CONCENTRATION_MEDIUM_PERCENT = 50.0
ELIGIBLE_CURRENCIES = {"ETH", "WETH", "USDC", "USDT", "DAI"}


def pool_metrics(pool: Mapping[str, Any]) -> dict[str, Any]:
    runtime_state = pool.get("runtime_state") or {}
    observed_buy = (
        (finite_number(runtime_state.get("denom_volume_in")) or 0.0) > 0.0
        and (finite_number(runtime_state.get("token_volume_out")) or 0.0) > 0.0
    )
    observed_sell = (
        (finite_number(runtime_state.get("token_volume_in")) or 0.0) > 0.0
        and (finite_number(runtime_state.get("denom_volume_out")) or 0.0) > 0.0
    )
    return {
        "currency": pool.get("currency"),
        "liquidity_level": pool.get("liquidity_level"),
        "denom_reserve": finite_number(pool.get("denom_reserve")),
        "token_reserve": finite_number(pool.get("token_reserve")),
        "total_liquidity": finite_number(pool.get("total_liquidity")),
        "eligible_liquidity_threshold": eligible_liquidity_threshold(pool),
        "eligible_liquidity": is_eligible_liquidity(pool),
        "raw_price_ratio_to_initial": finite_number(pool.get("raw_price_ratio_to_initial")),
        "price_ratio_to_initial": finite_number(pool.get("price_ratio_to_initial")),
        "pooled_token_supply_percent": finite_number(pool.get("pooled_token_supply_percent")),
        "liquidity_to_fdv_percent": finite_number(pool.get("liquidity_to_fdv_percent")),
        "buy_tax": finite_number(pool.get("buy_tax")),
        "sell_tax": finite_number(pool.get("sell_tax")),
        "tax_bucket": pool.get("tax_bucket"),
        "can_buy": pool.get("can_buy"),
        "can_sell": pool.get("can_sell"),
        "simulator_can_buy": runtime_state.get("can_buy"),
        "simulator_can_sell": runtime_state.get("can_sell"),
        "observed_buy": observed_buy,
        "observed_sell": observed_sell,
        "lp_supply_known": pool.get("lp_supply_known"),
        "lp_supply_status": pool.get("lp_supply_status"),
        "risk_level": pool.get("risk_level"),
        "risk_label": pool.get("risk_label"),
        "stage": pool.get("stage"),
        "pool_cohort": classification_value(pool, "cohort"),
        "pool_category": classification_value(pool, "category"),
        "eligible_outcome": classification_value(pool, "eligible_outcome"),
        "non_eligible_reason": classification_value(pool, "reason_key"),
    }


def contract_analysis_metrics(analysis: Mapping[str, Any]) -> dict[str, Any]:
    return {
        "interface_quality": nested_get(analysis, "interface", "quality"),
        "metadata_complete": nested_get(analysis, "interface", "metadata_complete"),
        "metadata": analysis.get("metadata"),
        "declared_total_supply_scaled": nested_get(
            analysis, "supply", "declared_total_supply_scaled"
        ),
        "minted_from_transfers": nested_get(analysis, "supply", "minted_from_transfers"),
        "minted_to_declared_ratio": nested_get(
            analysis, "supply", "minted_to_declared_ratio"
        ),
        "hidden_mint_detected": nested_get(analysis, "supply", "hidden_mint_detected"),
        "current_owner": nested_get(analysis, "authority", "current_owner"),
        "ownership_renounced": nested_get(analysis, "authority", "ownership_renounced"),
        "control_address_count": nested_get(
            analysis, "authority", "control_address_count"
        ),
        "pool_count": nested_get(analysis, "pools", "pool_count"),
        "trading_pool_count": nested_get(analysis, "pools", "trading_pool_count"),
        "cannot_sell_pool_count": nested_get(
            analysis, "pools", "cannot_sell_pool_count"
        ),
        "scam_pool_count": nested_get(analysis, "pools", "scam_pool_count"),
    }


def contract_analysis_next_step(kind: str) -> str:
    if kind == "hidden_mint_evidence":
        return "verify totalSupply, transfer mint events, and reserve/supply ratios before trusting FDV"
    if kind in {"metadata_incomplete", "invalid_metadata"}:
        return "treat as a nonstandard ERC20 case and separate chain behavior from metadata-read gaps"
    if kind == "raw_trading_event_without_pool_trading":
        return "keep token-level trading events separate from pool-derived trading viability"
    if kind in {"cannot_sell_pool", "pool_scam_evidence"}:
        return "replay observed chain routes and simulator setup for the affected pool"
    return "inspect the contract-analysis evidence and decide whether it needs a focused lab case"


def nested_get(value: Mapping[str, Any], *path: str) -> Any:
    current: Any = value
    for key in path:
        if not isinstance(current, Mapping):
            return None
        current = current.get(key)
    return current


def is_meaningfully_liquid(pool: Mapping[str, Any]) -> bool:
    value = finite_number(pool.get("denom_reserve")) or 0.0
    return value >= meaningful_liquidity_threshold(pool)


def is_low_liquidity(pool: Mapping[str, Any]) -> bool:
    value = finite_number(pool.get("denom_reserve")) or 0.0
    return value <= low_liquidity_threshold(pool)


def is_eligible_liquidity(pool: Mapping[str, Any]) -> bool:
    value = finite_number(pool.get("denom_reserve")) or 0.0
    return is_supported_eligibility_currency(pool) and value >= eligible_liquidity_threshold(pool)


def is_supported_eligibility_currency(pool: Mapping[str, Any]) -> bool:
    return str(pool.get("currency") or "").upper() in ELIGIBLE_CURRENCIES


def classification_label(pool: Mapping[str, Any]) -> str:
    reason = classification_value(pool, "reason_key")
    if reason:
        return reason
    outcome = classification_value(pool, "eligible_outcome")
    if outcome:
        return outcome
    category = classification_value(pool, "category")
    if category:
        return category
    cohort = classification_value(pool, "cohort")
    if cohort:
        return cohort

    if not is_supported_eligibility_currency(pool):
        return "unsupported_currency"
    if not is_eligible_liquidity(pool):
        return "low_liquidity"
    if not bool(pool.get("can_buy")):
        return "cannot_buy"
    if not bool(pool.get("can_sell")):
        return "cannot_sell"
    if str(pool.get("risk_level") or "").lower() in {"liquidity_removal", "honeypot"}:
        return "risk_blocked"
    return "eligible"


def pool_classification(pool: Mapping[str, Any]) -> Mapping[str, Any]:
    value = pool.get("pool_classification") or pool.get("poolClassification") or {}
    return value if isinstance(value, Mapping) else {}


def classification_value(pool: Mapping[str, Any], key: str) -> str:
    classification = pool_classification(pool)
    camel_key = snake_to_camel(key)
    value = (
        classification.get(key)
        or classification.get(camel_key)
        or pool.get(key)
        or pool.get(camel_key)
    )
    return str(value or "").strip()


def snake_to_camel(value: str) -> str:
    parts = value.split("_")
    return parts[0] + "".join(part.title() for part in parts[1:])


def looks_like_liquidity_drain(
    pool: Mapping[str, Any],
    current_liquidity: float,
    max_liquidity: float,
) -> bool:
    low_threshold = low_liquidity_threshold(pool)
    if max_liquidity <= low_threshold:
        return False
    if current_liquidity <= meaningful_liquidity_threshold(pool):
        return True
    if max_liquidity <= 0.0:
        return False
    return current_liquidity / max_liquidity <= 0.05


def meaningful_liquidity_threshold(pool: Mapping[str, Any]) -> float:
    currency = str(pool.get("currency") or "").upper()
    if currency in {"USDC", "USDT", "DAI"}:
        return STABLE_LIQUIDITY_DUST
    return WETH_LIQUIDITY_DUST


def eligible_liquidity_threshold(pool: Mapping[str, Any]) -> float:
    currency = str(pool.get("currency") or "").upper()
    if currency in {"USDC", "USDT", "DAI"}:
        return STABLE_LIQUIDITY_ELIGIBLE
    if currency in {"ETH", "WETH"}:
        return WETH_LIQUIDITY_ELIGIBLE
    return math.inf


def low_liquidity_threshold(pool: Mapping[str, Any]) -> float:
    currency = str(pool.get("currency") or "").upper()
    if currency in {"USDC", "USDT", "DAI"}:
        return STABLE_LIQUIDITY_LOW
    return WETH_LIQUIDITY_LOW


def finite_number(value: Any) -> float | None:
    if value is None or isinstance(value, bool):
        return None
    try:
        number = float(value)
    except (TypeError, ValueError):
        return None
    if not math.isfinite(number):
        return None
    return number


def as_int(value: Any) -> int | None:
    if value is None or isinstance(value, bool):
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def normalize_address(value: Any) -> str:
    if value is None:
        return ""
    return str(value).strip().lower()


def is_burn_address(value: str) -> bool:
    value = normalize_address(value)
    return value in {
        "",
        "0x0000000000000000000000000000000000000000",
        "0x000000000000000000000000000000000000dead",
    } or value.endswith("dead")


def opt_str(value: Any) -> str | None:
    if value is None:
        return None
    text = str(value).strip()
    return text or None


def metric_number(value: Any) -> str:
    number = finite_number(value)
    if number is None:
        return "-"
    if abs(number) >= 1_000_000 or (0 < abs(number) < 0.0001):
        return f"{number:.4e}"
    return f"{number:.4f}"


def compact_text(value: str, max_len: int = 120) -> str:
    value = " ".join(str(value).split())
    if len(value) <= max_len:
        return value
    return value[: max_len - 3] + "..."


def collapse_message(message: str) -> str:
    message = compact_text(message, 220)
    for marker in (" at block ", " block "):
        if marker in message:
            prefix, _, suffix = message.partition(marker)
            return f"{prefix}{marker}<block>{suffix[suffix.find(' '):] if ' ' in suffix else ''}".strip()
    return message


def looks_like_timeout(message: str) -> bool:
    lowered = message.lower()
    return "timed out" in lowered or "timeout" in lowered


def sample_range(label: str, values: Sequence[int]) -> str:
    if not values:
        return f"{label}=none"
    if len(values) == 1:
        return f"{label}={values[0]}"
    return f"{label}={values[0]}..{values[-1]} ({len(values)} unique)"


def sample_values(label: str, values: Sequence[str]) -> str:
    if not values:
        return f"{label}=none"
    return f"{label}={', '.join(short_hash(value) for value in values[:5])}"


def short_hash(value: str | None) -> str:
    if not value:
        return "-"
    value = str(value)
    if len(value) <= 12:
        return value
    return value[:6] + ".." + value[-6:]


def url_quote(value: str) -> str:
    return urllib.parse.quote(value, safe="")


def clean_json(value: Any) -> Any:
    if isinstance(value, dict):
        return {key: clean_json(item) for key, item in value.items() if item is not None}
    if isinstance(value, list):
        return [clean_json(item) for item in value]
    if isinstance(value, float):
        if not math.isfinite(value):
            return None
    return value
