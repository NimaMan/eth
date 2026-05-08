"""
Lightweight token snapshot helpers.

The live token cache holds full-blown ``ERC20Token`` objects with
mutable state, internal trackers, and bounded histories. Serialising
those objects wholesale is both expensive (lots of nested structures)
and fragile (non-JSON types, circular references).  Instead we expose a
curated dictionary snapshot that captures the fields external processes
care about: metadata, latest block context, pool status, control
addresses, tax information, and per-pool trading state.

Snapshot structure
------------------
The dict returned by :func:`build_token_snapshot` has these top-level sections:

``contract_address``: canonical address (checksum) of the token.

``metadata``: name, symbol, decimals, total_supply.

``creation``: creator address, block, timestamp, creation tx hash.

``latest_block``: block number / timestamp the token was last updated.

``status``: lifecycle enum, scam flags, ownership info, trading state.

``control``: current owner, renouncement info, control address list.

``pools``: for each DEX pool we track -> reserves, prices, trading
           flags, scam flags, pool ids (V4), LP approval stats.

``transfers``: aggregate bribe/tax info, per-address tx counters,
               approved addresses.

``transfer_history`` (optional): bounded history of ERC20 / ETH /
               other-denom transfers + approvals. Only populated when
               ``include_history=True`` to keep snapshots lean.

Call ``build_token_snapshot`` with an ``ERC20Token`` to obtain a JSON
friendly dict. Use :class:`TokenSnapshot` for typed access to an existing
snapshot payload.
"""

from dataclasses import dataclass
from typing import Any, Dict, Iterable, List, MutableMapping, Optional

from erc20_token.erc20_token import ERC20Token, TokenLifecycleState


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
            "latest_activity_block": token.latest_block_number,
        },
        "control": {
            "current_owner": token.current_owner,
            "ownership_renounced_block": token.ownership_renounced_block,
            "control_addresses": sorted(a for a in token.token_control_addresses if a),
            "tax_setter_addresses": sorted(getattr(token, "tax_setter_addresses", set())),
        },
        "pools": {
            "addresses": list(token.pool_addresses),
            "info": pool_info,
            "flags": {
                addr: _build_pool_flags(token, addr)
                for addr in token.pool_addresses or ()
            },
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


def build_token_snapshot_map(
    tokens: Dict[str, ERC20Token],
    *,
    include_history: bool = False,
    history_limit: int = DEFAULT_HISTORY_SNAPSHOT_LIMIT,
    on_error=None,
) -> Dict[str, Dict[str, Any]]:
    """Build Redis-friendly snapshots for a mapping of token address to token."""
    snapshots: Dict[str, Dict[str, Any]] = {}
    for address, token in (tokens or {}).items():
        try:
            snapshots[address] = build_token_snapshot(
                token,
                include_history=include_history,
                history_limit=history_limit,
            )
        except Exception as exc:
            if on_error is not None:
                on_error(address, exc)
            else:
                raise
    return snapshots


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


def _build_pool_flags(token: ERC20Token, pool_address: str) -> Dict[str, Any]:
    pool_manager = getattr(token, "pool_manager", None)
    pool_obj = pool_manager.get_pool(pool_address) if pool_manager else None
    return {
        "is_scam": getattr(pool_obj, "is_scam", False),
        "scam_label": getattr(pool_obj, "scam_label", None),
        "lifecycle": getattr(pool_obj, "lifecycle", None),
        "control_addresses": getattr(pool_obj, "control_addresses", []),
        "can_buy": getattr(pool_obj, "can_buy", False),
        "can_sell": getattr(pool_obj, "can_sell", False),
        "trading_enabled": getattr(pool_obj, "trading_enabled", False),
        "trading_enabled_block": getattr(pool_obj, "trading_enabled_block", None),
        "trading_enabled_tx": getattr(pool_obj, "trading_enabled_tx", None),
        "lp_tokens_approved_percentage": getattr(pool_obj, "lp_tokens_approved_percentage", None),
        "latest_block_number": getattr(pool_obj, "latest_block_number", None),
        "last_update_time": getattr(pool_obj, "last_update_time", None),
    }


__all__ = [
    "build_token_snapshot",
    "build_token_snapshot_map",
    "TokenSnapshot",
    "SNAPSHOT_VERSION",
    "DEFAULT_HISTORY_SNAPSHOT_LIMIT",
]
