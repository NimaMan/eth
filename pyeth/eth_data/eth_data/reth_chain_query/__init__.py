"""
Reth Chain Queries Module

This module provides blockchain query functionality using pyreth (Rust-based) 
implementation instead of Python RPC calls for improved performance.
"""

from .entities.stablecoin_queries import (
    get_stablecoin_total_supply,
    get_stablecoin_balance,
    get_all_stablecoins_total_supply,
    get_holder_all_stablecoin_balances,
    compare_top_stablecoins_supply,
)

from .reth_index.address_tx_history import (
    AddressTxRecord,
    RethAddressTxHistory,
)

__all__ = [
    "get_stablecoin_total_supply",
    "get_stablecoin_balance",
    "get_all_stablecoins_total_supply",
    "get_holder_all_stablecoin_balances",
    "compare_top_stablecoins_supply",
    "AddressTxRecord",
    "RethAddressTxHistory",
]
