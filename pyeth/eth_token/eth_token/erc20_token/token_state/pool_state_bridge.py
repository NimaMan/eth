"""Bridge between token runtime state and pool managers.

Responsibility:

* Host the ``PoolManager`` instance used for Uniswap V2/V3/V4 routing.
* Provide convenient accessors for pool collections, liquidity matrices,
  current prices, liquidity summaries, and price ratios.
* Forward every processed transaction to the manager and propagate
  control-address updates down to pools.
"""

from typing import Dict, Iterable, Iterator, List, Optional, Tuple, Union

from eth_token.erc20_token.pools import PoolManager
from eth_token.erc20_token.pools.base_pool import BasePool
from eth_token.erc20_token.pools.multi_pool_liquidity_analyzer import MultiPoolLiquidityAnalyzer


__all__ = ["PoolCollection", "PoolStateBridge"]


class PoolCollection:
    """List-like view over the pools managed by ``PoolManager``."""

    def __init__(self, manager: PoolManager):
        self._manager = manager

    def _all_pools(self) -> List[BasePool]:
        return self._manager.get_all_pools()

    def __len__(self) -> int:
        return len(self._all_pools())

    def __iter__(self) -> Iterator[BasePool]:
        return iter(self._all_pools())

    def __getitem__(self, key: Union[int, slice, str]) -> Union[BasePool, List[BasePool]]:
        pools = self._all_pools()
        if isinstance(key, slice):
            return pools[key]
        if isinstance(key, int):
            return pools[key]
        if isinstance(key, str):
            pool = self._manager.get_pool(key)
            if pool is None:
                raise KeyError(f"No pool found for key '{key}'")
            return pool
        raise TypeError(f"Unsupported key type {type(key)!r}")

    def __repr__(self) -> str:
        addresses = [
            getattr(pool, "pool_address", getattr(pool, "display_address", "<unknown>"))
            for pool in self._all_pools()
        ]
        return f"PoolCollection({addresses})"


class PoolStateBridge:
    """Encapsulates all PoolManager interactions for a token."""

    def __init__(
        self,
        *,
        token_address: str,
        history_limit: int,
        token_decimals: Optional[int] = None,
        pool_manager: Optional[PoolManager] = None,
    ) -> None:
        self.pool_manager = pool_manager or PoolManager(
            token_address=token_address,
            history_limit=history_limit,
        )
        self.set_token_decimals(token_decimals)
        self.pool_collection = PoolCollection(self.pool_manager)
        self.liquidity_matrix: "MultiPoolLiquidityAnalyzer" = MultiPoolLiquidityAnalyzer(self.pool_manager)

    def update_from_transaction(self, transaction: Dict) -> None:
        self.pool_manager.update_from_transaction(transaction)

    def set_token_decimals(self, decimals: int) -> None:
        self.pool_manager._token_decimals = int(decimals)

    def register_token_control_addresses(self, addresses: Iterable[Optional[str]]) -> None:
        self.pool_manager.register_token_control_addresses(addresses)

    def has_pools(self) -> bool:
        return self.pool_manager.has_pools()

    def get_pool_collection(self) -> PoolCollection:
        return self.pool_collection

    def get_liquidity_matrix(self):
        return self.liquidity_matrix

    def get_pool_addresses(self) -> Tuple[str, ...]:
        return tuple(self.pool_manager.get_all_pool_addresses())

    def get_pool(self, pool_address: str) -> Optional[BasePool]:
        return self.pool_manager.get_pool(pool_address)

    def get_all_pools(self) -> List[BasePool]:
        return self.pool_manager.get_all_pools()

    def get_pool_token_reserve(self, pool_address: str) -> Optional[float]:
        pool = self.pool_manager.get_pool(pool_address)
        if pool:
            return pool.get_token_reserve()
        return None

    def get_pool_denom_reserve(self, pool_address: str) -> Optional[float]:
        pool = self.pool_manager.get_pool(pool_address)
        if pool:
            return pool.get_denom_reserve()
        return None

    def get_total_liquidity_by_denom(self) -> Dict[str, float]:
        return self.pool_manager.get_total_liquidity()

    def get_pool_info(self) -> Dict:
        return self.pool_manager.get_pool_info()
    
    def get_pool_info_dict(self) -> Dict[str, Dict]:
        """Backwards-compatible alias for callers expecting the old name."""
        return self.get_pool_info()

    def get_all_pool_reserves(self) -> Dict[str, Dict[str, Union[float, str]]]:
        reserves: Dict[str, Dict[str, Union[float, str]]] = {}
        for pool in self.get_all_pools():
            reserves[pool.pool_address] = {
                "denom_reserve": pool.get_denom_reserve(),
                "token_reserve": pool.get_token_reserve(),
                "denom_symbol": pool.denom_address,
                "protocol": pool.get_protocol(),
            }
        return reserves

    def get_current_prices(self) -> Dict[str, Dict[str, Union[str, float]]]:
        prices: Dict[str, Dict[str, Union[str, float]]] = {}
        for pool in self.get_all_pools():
            price = pool.get_price()
            if price is not None and price > 0:
                prices[pool.pool_address] = {
                    "price": price,
                    "protocol": pool.get_protocol(),
                    "denom": pool.denom_address,
                }
        return prices

    def get_latest_price_ratios(self) -> Dict[str, float]:
        ratios: Dict[str, float] = {}
        for pool in self.get_all_pools():
            ratio = pool.reserve_tracker.get_price_ratio_to_initial()
            ratios[pool.pool_address] = ratio if ratio is not None else 0.0
        return ratios
