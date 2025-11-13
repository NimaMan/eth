"""
Lightweight token snapshot helpers.

The live token cache holds full-blown ``ERC20Token`` objects with
mutable state, internal trackers, and bounded histories. Serialising
those objects wholesale is both expensive (lots of nested structures)
and fragile (non-JSON types, circular references).  Instead we expose a
curated dictionary snapshot that captures the fields external processes
care about: metadata, latest block context, pool status, control
addresses, and a few summary metrics from the trackers.

Call ``build_token_snapshot`` with an ``ERC20Token`` to obtain a JSON
friendly dict. Heavy histories are omitted by default; pass
``include_history=True`` to include trimmed histories for debugging.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, Iterable, List, Mapping, MutableMapping, Optional

from .erc20_token import ERC20Token
from .erc20_token import TokenLifecycleState  # re-export for type hints

SNAPSHOT_VERSION = 1
DEFAULT_HISTORY_SNAPSHOT_LIMIT = 25


def build_token_snapshot(
    token: ERC20Token,
    *,
    include_history: bool = False,
    history_limit: int = DEFAULT_HISTORY_SNAPSHOT_LIMIT,
) -> Dict[str, Any]:
    """Return a dict representation of ``token`` suitable for external sharing.

    Args:
        token: The live ``ERC20Token`` instance.
        include_history: When True, include trimmed transfer/approval
            histories. Defaults to False to keep payloads small.
        history_limit: Max number of entries per history collection when
            ``include_history`` is enabled.
    """

    pool_info = token.get_pool_info_dict()

    snapshot: Dict[str, Any] = {
        "version": SNAPSHOT_VERSION,
        "contract_address": token.contract_address,
        "metadata": {
            "name": token.name,
            "symbol": token.symbol,
            "decimals": token.decimals,
            "total_supply": token.total_supply,
        },
        "creation": {
            "block": token.creation_block,
            "timestamp": token.creation_timestamp,
            "tx": token.creation_tx,
            "creator": token.creator_address,
            "creator_nonce": token.creator_nonce,
        },
        "latest_block": {
            "number": token.latest_block_number,
            "timestamp": token.latest_block_timestamp,
        },
        "status": {
            "lifecycle": _enum_value(token.token_life_cycle_status),
            "is_scam": token.is_scam,
            "has_pool": token.has_pool,
            "trading_enabled": token.trading_enabled,
            "trading_enabled_block": token.trading_enabled_block,
            "trading_enabled_tx": token.trading_enabled_tx,
            "ownership_renounced": token.ownership_renounced,
        },
        "control": {
            "current_owner": token.current_owner,
            "ownership_renounced_block": token.ownership_renounced_block,
            "control_addresses": sorted(a for a in token.token_control_addresses if a),
        },
        "pools": {
            "addresses": list(token.pool_addresses),
            "info": pool_info,
            "reserves": token.all_pool_reserves,
            "prices": token.current_prices,
            "total_liquidity_by_denom": token.total_liquidity_by_denom,
            "latest_price_ratios": token.latest_pools_price_ratio,
        },
        "transfers": {
            "total_bribe_amount": token.total_bribe_amount,
            "bribes_by_address": token.bribe_amounts_by_tx,
            "approved_addresses": sorted(token.transfer_tracker.approved_addresses),
            "address_tx_counter": dict(token.transfer_tracker.address_tx_counter),
        },
    }

    if include_history:
        snapshot["transfer_history"] = {
            "erc20": _trim_mapping(token.transfer_tracker.erc20_transfers, history_limit),
            "eth": _trim_mapping(token.transfer_tracker.eth_transfers, history_limit),
            "other_denoms": _trim_mapping(token.transfer_tracker.other_denom_transfers, history_limit),
            "approvals": _trim_list(token.transfer_tracker.approvals, history_limit),
        }

    return snapshot


@dataclass
class TokenSnapshot:
    contract_address: str
    metadata: Dict[str, Any]
    creation: Dict[str, Any]
    latest_block: Dict[str, Any]
    status: Dict[str, Any]
    control: Dict[str, Any]
    pools: Dict[str, Any]
    transfers: Dict[str, Any]
    transfer_history: Optional[Dict[str, Any]] = None
    version: int = SNAPSHOT_VERSION

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "TokenSnapshot":
        return cls(
            contract_address=data.get("contract_address", ""),
            metadata=dict(data.get("metadata") or {}),
            creation=dict(data.get("creation") or {}),
            latest_block=dict(data.get("latest_block") or {}),
            status=dict(data.get("status") or {}),
            control=dict(data.get("control") or {}),
            pools=dict(data.get("pools") or {}),
            transfers=dict(data.get("transfers") or {}),
            transfer_history=data.get("transfer_history"),
            version=data.get("version", SNAPSHOT_VERSION),
        )

    def to_dict(self) -> Dict[str, Any]:
        payload = {
            "version": self.version,
            "contract_address": self.contract_address,
            "metadata": self.metadata,
            "creation": self.creation,
            "latest_block": self.latest_block,
            "status": self.status,
            "control": self.control,
            "pools": self.pools,
            "transfers": self.transfers,
        }
        if self.transfer_history is not None:
            payload["transfer_history"] = self.transfer_history
        return payload


def load_token_snapshot(
    token_address: str,
    reader: Optional["LiveDataReader"] = None,
) -> Optional[TokenSnapshot]:
    """
    Fetch the latest snapshot for ``token_address`` from the live data registry.
    """
    if reader is None:
        from eth_data.live_data_registry import LiveDataReader

        reader = LiveDataReader()
    snapshot = reader.get_token_snapshot(token_address)
    if snapshot is None:
        return None
    return TokenSnapshot.from_dict(snapshot)


def _enum_value(value: Optional[TokenLifecycleState]) -> Optional[str]:
    return value.value if value is not None else None


def _trim_list(items: Iterable[Dict[str, Any]], limit: int) -> List[Dict[str, Any]]:
    if limit <= 0:
        return list(items)
    return list(items)[-limit:]


def _trim_mapping(
    mapping: MutableMapping[str, List[Dict[str, Any]]],
    limit: int,
) -> Dict[str, List[Dict[str, Any]]]:
    trimmed: Dict[str, List[Dict[str, Any]]] = {}
    for key, values in mapping.items():
        trimmed[key] = _trim_list(values, limit)
    return trimmed


__all__ = [
    "build_token_snapshot",
    "TokenSnapshot",
    "load_token_snapshot",
    "SNAPSHOT_VERSION",
    "DEFAULT_HISTORY_SNAPSHOT_LIMIT",
]
