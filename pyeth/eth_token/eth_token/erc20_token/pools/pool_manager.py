"""
Pool Manager: Comprehensive Uniswap Pool Management for a Single ERC20 Token.

This module provides the `PoolManager` class, a centralized controller for discovering,
tracking, and interacting with Uniswap V2, V3, and V4 liquidity pools associated with a specific ERC20 token.

Core Functionalities:
1.  **Dynamic Pool Discovery**:
    -   **Creation Events**: Actively listens for pool creation events (e.g., UniswapV2's `PairCreated`,
        V3's `PoolCreated`, V4's `PoolInitialized`) within processed transactions to automatically
        register new liquidity pools as they are created on-chain.
    -   **Swap-Based Discovery**: Retroactively identifies and registers pre-existing pools by observing
        swap events. If a swap occurs in a pool not yet tracked, the manager queries the blockchain
        to fetch pool details (like token pairs and fees) and instantiates the appropriate pool object. This
        ensures that pools created before monitoring began are not missed.

2.  **Transaction Routing**:
    -   The main entry point, `process_transaction`, orchestrates the entire flow. It first checks for
        new pools, then routes the transaction data to all relevant, registered pool instances
        (V2, V3, and V4). Each pool object is then responsible for processing the transaction to
        update its internal state (e.g., reserves, prices).

3.  **State Management & Data Aggregation**:
    -   Maintains a structured inventory of all pools, indexed by address, protocol (V2/V3/V4), and
        denomination token for efficient lookups.
    -   Provides aggregated views and statistics across all managed pools, such as total liquidity
        per denomination currency, total swap volume, and health status.
    -   Offers methods to retrieve pool information in formats compatible with other system components.

4.  **Manual & Deterministic Pool Handling**:
    -   Supports manual addition of pools via `add_pool`.
    -   Includes utilities to compute the deterministic addresses for Uniswap V2 and V3 pools
        based on token pairs and fees.

Interaction with Blockchain:
-   To support swap-based discovery, the `PoolManager` can be configured with a Web3 provider.
    It uses this connection to make on-chain calls (`token0()`, `token1()`, `fee()`) to gather
    the necessary data for instantiating pools that were not discovered through creation events.
"""

from typing import Dict, List, Optional, Tuple, Any, Iterable, Set
from collections import defaultdict
from web3 import Web3

from .base_pool import BasePool
from .uniswap_v2_pool import UniswapV2Pool
from .uniswap_v3_pool import UniswapV3Pool
from .uniswap_v4_pool import UniswapV4Pool, PoolKey
from eth_token.erc20_token.pools.pool_chain_data_fetcher import PoolChainDataFetcher
from eth_token.erc20_token.data.token_chain_data_fetcher import TokenChainDataFetcher

from eth_data.chain_utils.common_addresses import (
    DENOM_ADDRESSES,
    canonicalize_dex_pool_type,
)


UNISWAP_V2_PROTOCOL = canonicalize_dex_pool_type('UNISWAP-V2')
UNISWAP_V3_PROTOCOL = canonicalize_dex_pool_type('UNISWAP-V3')
UNISWAP_V4_PROTOCOL = canonicalize_dex_pool_type('UNISWAP-V4')


PROTOCOL_ALIASES = {
    "V2": UNISWAP_V2_PROTOCOL,
    "V3": UNISWAP_V3_PROTOCOL,
    "V4": UNISWAP_V4_PROTOCOL,
}


class PoolManager:
    """
    Manages all pools for a token.
    
    Routes transaction events to the appropriate pool instances
    and provides aggregated views across all pools.
    """
    
    def __init__(self, token_address: str, history_limit: int = 100):
        self.token_address = token_address
        self.history_limit = history_limit
        
        # Pool storage by address
        self.pools: Dict[str, BasePool] = {}
        # Track pools being processed to avoid race conditions
        self._processing_pools: set = set()
        
        # Indexes for quick lookup
        self.pools_by_protocol: Dict[str, List[str]] = defaultdict(list)
        self.pools_by_denom: Dict[str, List[str]] = defaultdict(list)
        
        # V4 pools tracked separately by PoolId
        self.v4_pools: Dict[str, UniswapV4Pool] = {}  # poolId -> V4Pool                
        # Initialize pool chain data fetcher
        self.chain_data_fetcher = PoolChainDataFetcher()
        self.token_chain_data_fetcher = TokenChainDataFetcher()

        # Cached decimals to avoid repeat lookups
        self._token_decimals: Optional[int] = None
        self._denom_decimals_cache: Dict[str, int] = {}
        self._token_control_addresses: Set[str] = set()

    def _get_token_decimals(
        self,
        block_number: Optional[int] = None,
        block_header: Optional[str] = None,
    ) -> Optional[int]:
        if self._token_decimals is None:
            self._token_decimals = int(
                self.token_chain_data_fetcher.get_token_decimals(
                    self.token_address,
                    block_number,
                    block_header,
                )
            )
        return self._token_decimals

    def _get_denom_decimals(
        self,
        denom_address: str,
        block_number: Optional[int] = None,
        block_header: Optional[str] = None,
    ) -> int:
        checksum_address = Web3.to_checksum_address(denom_address)
        if checksum_address not in self._denom_decimals_cache:
            self._denom_decimals_cache[checksum_address] = int(
                self.token_chain_data_fetcher.get_token_decimals(
                    checksum_address,
                    block_number,
                    block_header,
                )
            )
        return self._denom_decimals_cache[checksum_address]

    def _pool_decimal_kwargs(
        self,
        denom_address: str,
        *,
        block_number: Optional[int] = None,
        block_header: Optional[str] = None,
    ) -> Dict[str, int]:
        return {
            'token_decimals': self._get_token_decimals(block_number, block_header),
            'denom_decimals': self._get_denom_decimals(
                denom_address,
                block_number,
                block_header,
            ),
            'history_limit': self.history_limit,
        }

    def process_transaction(self, transaction: Dict):
        """
        Process a transaction and route events to appropriate pools.
        
        First checks for new pool creation events, then checks swap events
        for pools we might have missed, then routes other events to existing pools.
        """
        # Check for new pool creations
        self._check_pool_creations(transaction)
        
        # Check swap events for pools we haven't seen yet
        self._check_swap_events_for_pools(transaction)
        
        # Route events to existing pools
        for pool in self.pools.values():
            pool.update_latest_block_transactions(transaction)
            pool.process_transaction(transaction)
            
        # Route V4 events to V4 pools
        for v4_pool in self.v4_pools.values():
            v4_pool.update_latest_block_transactions(transaction)
            v4_pool.process_transaction(transaction)            
            
    def _check_pool_creations(self, transaction: Dict):
        """Check for new pool creation events."""
        # V2 pair creation
        if transaction.get('uniswap_v2_pair_created_events'):
            for pair_event in transaction['uniswap_v2_pair_created_events']:
                self._handle_v2_creation(pair_event, transaction)
            
        # V3 pool creation
        if transaction.get('uniswap_v3_pools'):
            for pool_event in transaction['uniswap_v3_pools']:
                self._handle_v3_creation(pool_event, transaction)
            
        # V4 pool initialization (different pattern - uses PoolId)
        if transaction.get('uniswap_v4_initializes'):
            for init_event in transaction['uniswap_v4_initializes']:
                self._handle_v4_initialization(init_event, transaction)
        
    def _handle_v2_creation(self, pair_event: dict, transaction: Dict):
        """Handle V2 pair creation."""

        pair_address = pair_event['pair_address']
        token0 = pair_event['token0']
        token1 = pair_event['token1']
        
        # Skip if already registered
        if pair_address in self.pools:
            return
            
        # Check if our token is involved
        if token0 == self.token_address:
            denom_address = token1
            token1_is_denom = True
        elif token1 == self.token_address:
            denom_address = token0
            token1_is_denom = False
        else:
            # Our token not in this pair
            return
            
        # Create V2 pool instance
        decimal_kwargs = self._pool_decimal_kwargs(
            denom_address,
            block_number=transaction.get('block_number'),
            block_header=transaction.get('block_header'),
        )
        pool = UniswapV2Pool(
            pool_address=pair_address,
            token_address=self.token_address,
            denom_address=denom_address,
            **decimal_kwargs,
            token1_is_denom=token1_is_denom,
            pool_chain_fetcher=self.chain_data_fetcher,
            token_chain_fetcher=self.token_chain_data_fetcher,
        )
        
        pool.creation_block = transaction['block_number']
        pool.creation_tx = transaction['hash']
        pool.creation_timestamp = transaction['block_timestamp']
        self._register_pool(pool)
        
    def _handle_v3_creation(self, pool_event: dict, transaction: Dict):
        """Handle V3 pool creation."""
        
        # V3 pool creation events use 'pool' field for address
        pool_address = pool_event.get('pool', pool_event.get('pool_address', ''))
        token0 = pool_event['token0']
        token1 = pool_event['token1']
        fee = pool_event.get('fee', 3000)
        
        # Skip if already registered
        if pool_address in self.pools:
            return
            
        # Check if our token is involved
        if token0 == self.token_address:
            denom_address = token1
            token1_is_denom = True
        elif token1 == self.token_address:
            denom_address = token0
            token1_is_denom = False
        else:
            return
            
        # Create V3 pool instance
        decimal_kwargs = self._pool_decimal_kwargs(
            denom_address,
            block_number=transaction.get('block_number'),
            block_header=transaction.get('block_header'),
        )
        pool = UniswapV3Pool(
            pool_address=pool_address,
            token_address=self.token_address,
            denom_address=denom_address,
            **decimal_kwargs,
            token1_is_denom=token1_is_denom,
            fee_tier=fee,
            pool_chain_fetcher=self.chain_data_fetcher,
            token_chain_fetcher=self.token_chain_data_fetcher,
        )
        
        pool.creation_block = transaction['block_number']
        pool.creation_tx = transaction['hash']
        pool.creation_timestamp = transaction['block_timestamp']
        self._register_pool(pool)
        
    def _handle_v4_initialization(self, init_event: dict, transaction: Dict):
        """Handle V4 pool initialization.
        
        V4 pools are identified by PoolId, not address.
        All V4 pools share the same PoolManager address.
        """
        
        # V4 events use event_id for pool identification
        pool_id = init_event.get('event_id', init_event.get('pool_id', ''))
        currency0 = init_event['currency0']
        currency1 = init_event['currency1']
        fee = init_event.get('fee', 3000)
        tick_spacing = init_event.get('tick_spacing', 60)
        hooks = init_event.get('hooks', '0x0000000000000000000000000000000000000000')
        
        # Skip if already registered
        if pool_id in self.v4_pools:
            return
            
        # Check if our token is involved
        if currency0 == self.token_address:
            denom_address = currency1
            token1_is_denom = True
        elif currency1 == self.token_address:
            denom_address = currency0
            token1_is_denom = False
        else:
            return
            
        # Create pool key
        pool_key = PoolKey(
            currency0=currency0,
            currency1=currency1,
            fee=fee,
            tick_spacing=tick_spacing,
            hooks=hooks
        )
        
        # Create V4 pool instance
        decimal_kwargs = self._pool_decimal_kwargs(
            denom_address,
            block_number=transaction.get('block_number'),
            block_header=transaction.get('block_header'),
        )
        pool = UniswapV4Pool(
            pool_id=pool_id,
            pool_key=pool_key,
            token_address=self.token_address,
            denom_address=denom_address,
            **decimal_kwargs,
            token1_is_denom=token1_is_denom,
            pool_chain_fetcher=self.chain_data_fetcher,
            token_chain_fetcher=self.token_chain_data_fetcher,
        )
        
        pool.creation_block = transaction['block_number']
        pool.creation_tx = transaction['hash']
        pool.creation_timestamp = transaction['block_timestamp']
        self._register_v4_pool(pool)
        
    def _register_pool(self, pool: BasePool):
        """Register a pool in the manager."""
        self.pools[pool.pool_address] = pool
        protocol_name = pool.get_protocol()
        self.pools_by_protocol[protocol_name].append(pool.pool_address)
        self.pools_by_denom[pool.denom_address].append(pool.pool_address)
        
    def _register_v4_pool(self, pool: UniswapV4Pool):
        """Register a V4 pool in the manager. V4 pools are tracked separately by PoolId."""
        self.v4_pools[pool.pool_id] = pool
        self.pools_by_protocol[UNISWAP_V4_PROTOCOL].append(pool.pool_id)
        self.pools_by_denom[pool.denom_address].append(pool.pool_id)

    def register_token_control_addresses(self, addresses: Iterable[Optional[str]]) -> None:
        for pool in self.get_all_pools():
            pool.register_token_control_addresses(addresses)
    
    def add_pool(self, pool_address: str, protocol: str, denom_address: str, 
                 token1_is_denom: bool = True, **kwargs):
        """
        Manually add a pool.
        
        Used for pools that already exist when we start tracking a token.
        """
        
        # Skip if already exists
        if pool_address in self.pools:
            return self.pools[pool_address]

        normalized_protocol = canonicalize_dex_pool_type(
            PROTOCOL_ALIASES.get(protocol, protocol)
        )

        # Create appropriate pool instance
        if normalized_protocol == UNISWAP_V2_PROTOCOL:
            decimal_kwargs = self._pool_decimal_kwargs(denom_address)
            pool = UniswapV2Pool(
                pool_address=pool_address,
                token_address=self.token_address,
                denom_address=denom_address,
                **decimal_kwargs,
                token1_is_denom=token1_is_denom,
                pool_chain_fetcher=self.chain_data_fetcher,
                token_chain_fetcher=self.token_chain_data_fetcher,
            )
        elif normalized_protocol == UNISWAP_V3_PROTOCOL:
            decimal_kwargs = self._pool_decimal_kwargs(denom_address)
            pool = UniswapV3Pool(
                pool_address=pool_address,
                token_address=self.token_address,
                denom_address=denom_address,
                **decimal_kwargs,
                token1_is_denom=token1_is_denom,
                fee_tier=kwargs.get('fee_tier', 3000),
                pool_chain_fetcher=self.chain_data_fetcher,
                token_chain_fetcher=self.token_chain_data_fetcher,
            )
        elif normalized_protocol == UNISWAP_V4_PROTOCOL:
            # V4 requires pool_id instead of pool_address
            pool_id = kwargs.get('pool_id')
            if not pool_id:
                raise ValueError("V4 pools require pool_id parameter")

            return self.add_v4_pool(
                pool_id=pool_id,
                denom_address=denom_address,
                token1_is_denom=token1_is_denom,
                **kwargs
            )
        else:
            raise ValueError(f"Unknown protocol {protocol} for pool {pool_address} and token {self.token_address}")
            
        self._register_pool(pool)
        return pool
    
    def add_v4_pool(self, pool_id: str, denom_address: str, 
                    token1_is_denom: bool = True, **kwargs):
        """
        Manually add a V4 pool.
        
        Args:
            pool_id: The V4 pool ID (bytes32 hash)
            denom_address: The paired token address
            token1_is_denom: Whether token1 is the denomination token
            **kwargs: Additional parameters (fee, tick_spacing, hooks)
        
        Returns:
            The created V4 pool instance or None if creation fails
        """
        # Check if already exists
        if pool_id in self.v4_pools:
            return self.v4_pools[pool_id]
        
        # Create PoolKey
        pool_key = PoolKey(
            currency0=self.token_address if not token1_is_denom else denom_address,
            currency1=denom_address if not token1_is_denom else self.token_address,
            fee=kwargs.get('fee', 3000),
            tick_spacing=kwargs.get('tick_spacing', 60),
            hooks=kwargs.get('hooks', '0x0000000000000000000000000000000000000000')
        )
        
        # Create V4 pool instance
        decimal_kwargs = self._pool_decimal_kwargs(denom_address)
        pool = UniswapV4Pool(
            pool_id=pool_id,
            pool_key=pool_key,
            token_address=self.token_address,
            denom_address=denom_address,
            **decimal_kwargs,
            token1_is_denom=token1_is_denom,
            pool_chain_fetcher=self.chain_data_fetcher,
            token_chain_fetcher=self.token_chain_data_fetcher,
        )
        
        self._register_v4_pool(pool)
        return pool
        
    def get_pool(self, pool_address: str) -> Optional[BasePool]:
        """Get a specific pool by address or display address."""
        # Support V4 display address formats (POOL_MANAGER-prefix or PoolManager#id)
        if '-' in pool_address:
            prefix, _, pool_id = pool_address.partition('-')
            if prefix == UniswapV4Pool.POOL_MANAGER and pool_id:
                return self.v4_pools.get(pool_id)

        # Check if it's a V4 display address (PoolManager#poolId)
        if '#' in pool_address:
            parts = pool_address.split('#')
            if len(parts) == 2:
                pool_id = parts[1]
                return self.v4_pools.get(pool_id)
        pool = self.pools.get(Web3.to_checksum_address(pool_address))
        return pool
        
    def get_pools_by_protocol(self, protocol: str) -> List[BasePool]:
        """Get all pools for a specific protocol."""
        normalized_protocol = canonicalize_dex_pool_type(
            PROTOCOL_ALIASES.get(protocol, protocol)
        )
        addresses = self.pools_by_protocol.get(normalized_protocol, [])
        return [self.pools[addr] for addr in addresses]
        
    def get_pools_by_denom(self, denom_address: str) -> List[BasePool]:
        """Get all pools paired with a specific denomination token."""
        addresses = self.pools_by_denom.get(denom_address, [])
        return [self.pools[addr] for addr in addresses]
        
    def get_all_pools(self) -> List[BasePool]:
        """Get all pools (V2, V3, and V4)."""
        all_pools = list(self.pools.values())
        all_pools.extend(self.v4_pools.values())
        return all_pools
        
    def get_total_liquidity(self) -> Dict[str, float]:
        """Get total liquidity across all pools by denomination."""
        liquidity_by_denom = defaultdict(float)
        
        for pool in self.pools.values():
            denom_reserve = pool.get_denom_reserve()
            liquidity_by_denom[pool.denom_address] += denom_reserve
            
        # Include V4 pools
        for pool in self.v4_pools.values():
            denom_reserve = pool.get_denom_reserve()
            liquidity_by_denom[pool.denom_address] += denom_reserve
            
        return dict(liquidity_by_denom)
        
    def get_pool_health_stats(self) -> Dict[str, Dict]:
        """
        Get health statistics for all pools including scam status.
        
        Returns dict mapping pool addresses to health info.
        """
        health_stats = {}
        
        for address, pool in self.pools.items():
            health_stats[address] = {
                'pool_type': pool.get_protocol(),
                'is_scam': bool(pool.scam_label),
                'scam_label': pool.scam_label,
                'scam_block': pool.scam_block,
                'scam_tx_hash': pool.scam_tx_hash,
                'denom_reserve': pool.get_denom_reserve(),
                'token_reserve': pool.get_token_reserve(),
                'price': pool.get_price(),
                'has_liquidity': pool.get_denom_reserve() > 0,
            }
            
        # Add V4 pools
        for pool_id, pool in self.v4_pools.items():
            health_stats[pool.display_address] = {
                'pool_type': UNISWAP_V4_PROTOCOL,
                'pool_id': pool_id,
                'is_scam': bool(pool.scam_label),
                'scam_label': pool.scam_label,
                'scam_block': pool.scam_block,
                'scam_tx_hash': pool.scam_tx_hash,
                'denom_reserve': pool.get_denom_reserve(),
                'token_reserve': pool.get_token_reserve(),
                'price': pool.get_price(),
                'has_liquidity': pool.get_denom_reserve() > 0, 
            }
            
        return health_stats
        
    def get_stats(self) -> Dict:
        """Get aggregated statistics across all pools."""
        # Count pools including V4
        total_pools = len(self.pools) + len(self.v4_pools)
        
        # Count healthy pools (those without scam labels)
        healthy_pools = sum(1 for p in self.pools.values() if not p.scam_label)
        healthy_pools += sum(1 for p in self.v4_pools.values() if not p.scam_label)
        
        # Count scam pools
        scam_pools = sum(1 for p in self.pools.values() if p.scam_label)
        scam_pools += sum(1 for p in self.v4_pools.values() if p.scam_label)
        
        stats = {
            'total_pools': total_pools,
            'pools_by_protocol': {
                protocol: len(addresses) 
                for protocol, addresses in self.pools_by_protocol.items()
            },
            'healthy_pools': healthy_pools,
            'scam_pools': scam_pools,
            'total_swaps': sum(p.state.total_swaps for p in self.pools.values()),
            'total_liquidity_by_denom': self.get_total_liquidity(),
        }
        
        return stats
            
    def get_pool_info(self) -> Dict[str, Dict]:
        """
        Get pool information in LiveTokenData format.
        
        Returns dict compatible with LiveTokenData.pool_info structure:
        {pool_address: {pool_type, denom_currency, decimals, token1_is_denom}}
        """
        pool_info = {}
        
        # Add V2/V3 pools
        for address, pool in self.pools.items():
            denom_symbol = self._get_denom_symbol(pool.denom_address)
            token_decimals = pool.get_token_decimals()
            denom_decimals = pool.get_denom_decimals()

            pool_data = {
                'pool_type': pool.get_protocol(),
                'denom_address': pool.denom_address,
                'denom_currency': denom_symbol,
                'token_decimals': token_decimals,
                'denom_decimals': denom_decimals,
                'token_reserve': pool.get_token_reserve(),
                'denom_reserve': pool.get_denom_reserve(),
                'price': pool.get_price(),
                'price_ratio': pool.reserve_tracker.get_price_ratio_to_initial(),
                'scam_tx': pool.scam_tx_hash,
                'trading_enabled': pool.trading_enabled,
                'trading_enabled_block': pool.trading_enabled_block,
                'buy_tax': pool.buy_tax,
                'sell_tax': pool.sell_tax
            }
            
            # Add LP holder information and approval percentage for V2 pools
            if pool.get_protocol() == UNISWAP_V2_PROTOCOL:
                lp_holders = pool.get_lp_holders()
                if lp_holders:
                    pool_data['lp_holders'] = lp_holders
                # Add LP approval percentage
                pool_data['lp_tokens_approved_percentage'] = pool.get_lp_approved_percentage()
                # Add last LP approval information
                last_approval_event = pool.get_last_lp_approval_event()
                pool_data['lp_last_approval_block'] = (
                    last_approval_event.get('block_number') if last_approval_event else None
                )
                pool_data['lp_last_approval'] = last_approval_event
                    
            pool_info[address] = pool_data
            
        # Add V4 pools (use display address for compatibility)
        for pool_id, pool in self.v4_pools.items():
            v4_token_decimals = pool.get_token_decimals()
            v4_denom_decimals = pool.get_denom_decimals()

            pool_info[pool.display_address] = {
                'pool_type': UNISWAP_V4_PROTOCOL,
                'denom_address': pool.denom_address,
                'denom_currency': self._get_denom_symbol(pool.denom_address),
                'token_decimals': v4_token_decimals,
                'denom_decimals': v4_denom_decimals,
                'token_reserve': pool.get_token_reserve(),
                'denom_reserve': pool.get_denom_reserve(),
                'price': pool.get_price(),
                'price_ratio': pool.reserve_tracker.get_price_ratio_to_initial(),
                'scam_tx': pool.scam_tx_hash,
                'pool_id': pool_id,  # Extra field for V4
                'trading_enabled': pool.trading_enabled,
                'trading_enabled_block': pool.trading_enabled_block,
                'buy_tax': pool.buy_tax,
                'sell_tax': pool.sell_tax
            }
            
        return pool_info
        
    def get_all_pool_addresses(self) -> List[str]:
        """
        Get all pool addresses including V4 display addresses.
        For V4 pools, returns the display address (PoolManager#poolId)
        """
        addresses = list(self.pools.keys())
        
        # Add V4 display addresses
        for pool in self.v4_pools.values():
            addresses.append(pool.display_address)
            
        return addresses
        
    def get_denom_currencies(self) -> Dict[str, str]:
        """Get mapping of pool address -> denom currency name."""
        currencies = {}
        
        for address, pool in self.pools.items():
            currencies[address] = self._get_denom_symbol(pool.denom_address)
            
        for pool in self.v4_pools.values():
            currencies[pool.display_address] = self._get_denom_symbol(pool.denom_address)
            
        return currencies
        
    def get_trading_enabled_pools(self) -> List[Dict[str, Any]]:
        """
        Get list of pools where trading is enabled.
        
        Returns:
            List of dicts with pool info including address, protocol, and tax rates
        """
        enabled_pools = []
        
        # Check regular pools (V2/V3)
        for address, pool in self.pools.items():
            if pool.trading_enabled:
                enabled_pools.append({
                    'address': address,
                    'protocol': pool.get_protocol(),
                    'buy_tax': pool.buy_tax,
                    'sell_tax': pool.sell_tax,
                    'trading_enabled_block': pool.trading_enabled_block,
                    'denom': self._get_denom_symbol(pool.denom_address)
                })
        
        # Check V4 pools
        for pool_id, pool in self.v4_pools.items():
            if pool.trading_enabled:
                enabled_pools.append({
                    'address': pool.display_address,
                    'protocol': UNISWAP_V4_PROTOCOL,
                    'buy_tax': pool.buy_tax,
                    'sell_tax': pool.sell_tax,
                    'trading_enabled_block': pool.trading_enabled_block,
                    'denom': self._get_denom_symbol(pool.denom_address)
                })
                
        return enabled_pools
    
    def get_pool_taxes(self) -> Dict[str, Tuple[Optional[float], Optional[float]]]:
        taxes = {}
        
        # Regular pools
        for address, pool in self.pools.items():
            taxes[address] = (pool.buy_tax, pool.sell_tax)
            
        # V4 pools
        for pool in self.v4_pools.values():
            taxes[pool.display_address] = (pool.buy_tax, pool.sell_tax)
            
        return taxes
    
    def has_pools(self) -> bool:
        """Check if token has any pools."""
        return len(self.pools) > 0 or len(self.v4_pools) > 0
    
    def get_total_lp_supply(self) -> Dict[str, float]:
        """Get total LP supply for all V2 pools.
        
        Returns:
            Dict mapping pool address to LP token total supply
        """
        lp_supplies = {}
        for address, pool in self.pools.items():
            if pool.get_protocol() == UNISWAP_V2_PROTOCOL:
                lp_supplies[address] = pool.lp_total_supply
        return lp_supplies
    
    def _check_swap_events_for_pools(self, transaction: Dict):
        """
        Check swap events for pools we haven't seen yet.
        
        This is useful when:
        1. We start monitoring a token that already has pools
        2. Pool creation events are missing from transactions
        3. We want to discover pools from trading activity
        """
        # Check V2 swaps
        if transaction.get('uniswap_v2_swaps'):
            for swap in transaction['uniswap_v2_swaps']:
                pair_address = swap.get('pair_address', '')
                if pair_address and pair_address not in self.pools:
                    self._load_and_register_v2_pool(pair_address, transaction)
                
        # Check V3 swaps  
        if transaction.get('uniswap_v3_swaps'):
            for swap in transaction['uniswap_v3_swaps']:
                pool_address = swap.get('pool_address', '')
                if pool_address and pool_address not in self.pools:
                    self._load_and_register_v3_pool(pool_address, transaction)
                
    def _load_and_register_v2_pool(self, pair_address: str, transaction: Dict):
        """
        Load V2 pool configuration from the blockchain and register it.

        Used when a pool is detected through swap events rather than creation events.
        """
        try:
            # Skip if already being processed
            if pair_address in self._processing_pools:
                return
                
            self._processing_pools.add(pair_address)
            # Use chain data fetcher to get pool info
            pool_info = self.chain_data_fetcher.fetch_pool_metadata_for_token(
                pool_address=pair_address,
                token_address=self.token_address,
                protocol_hint=UNISWAP_V2_PROTOCOL,
                block_number=transaction.get('block_number'),
                block_header=transaction.get('block_header', None),
            )
            
            if pool_info is None:
                return
            
            # Create pool instance
            decimal_kwargs = self._pool_decimal_kwargs(
                pool_info['denom_address'],
                block_number=transaction.get('block_number'),
                block_header=transaction.get('block_header', None),
            )
            pool = UniswapV2Pool(
                pool_address=pair_address,
                token_address=self.token_address,
                denom_address=pool_info['denom_address'],
                **decimal_kwargs,
                token1_is_denom=pool_info['token1_is_denom']
            )
            
            pool.creation_block = transaction.get('block_number')
            pool.creation_tx = transaction.get('hash')
            pool.creation_timestamp = transaction.get('block_timestamp')
            pool.detected_from_swap = True
            
            # Register pool
            self._register_pool(pool)
            
        except Exception as e:
            raise RuntimeError(
                "PoolManager._load_and_register_v2_pool failed: "
                f"pair={pair_address} token={self.token_address} error={e}"
            ) from e
        finally:
            self._processing_pools.discard(pair_address)
            
    def _load_and_register_v3_pool(self, pool_address: str, transaction: Dict):
        """
        Load V3 pool configuration from the blockchain and register it.

        Used when a pool is detected through swap events rather than creation events.
        """
        try:
            # Skip if already being processed
            if pool_address in self._processing_pools:
                return
                
            self._processing_pools.add(pool_address)
            
            # Use chain data fetcher to get pool info
            pool_info = self.chain_data_fetcher.fetch_pool_metadata_for_token(
                pool_address=pool_address,
                token_address=self.token_address,
                protocol_hint=UNISWAP_V3_PROTOCOL,
                block_number=transaction.get('block_number'),
                block_header=transaction.get('block_header'),
            )
            
            if pool_info is None:
                return
            
            # Create pool instance
            decimal_kwargs = self._pool_decimal_kwargs(
                pool_info['denom_address'],
                block_number=transaction.get('block_number'),
                block_header=transaction.get('block_header'),
            )
            pool = UniswapV3Pool(
                pool_address=pool_address,
                token_address=self.token_address,
                denom_address=pool_info['denom_address'],
                **decimal_kwargs,
                token1_is_denom=pool_info['token1_is_denom'],
                fee_tier=pool_info.get('fee', 3000)
            )
            
            pool.creation_block = transaction.get('block_number')
            pool.creation_tx = transaction.get('hash')
            pool.creation_timestamp = transaction.get('block_timestamp')
            pool.detected_from_swap = True
            
            # Register pool
            self._register_pool(pool)
            
        except Exception as e:
            raise RuntimeError(
                "PoolManager._load_and_register_v3_pool failed: "
                f"pool={pool_address} token={self.token_address} "
                f"block={transaction.get('block_number')} tx={transaction.get('hash')} error={e}"
            ) from e
        finally:
            self._processing_pools.discard(pool_address)
    
    def _get_denom_symbol(self, token_address: str) -> str:
        return DENOM_ADDRESSES.get(token_address, 'Unknown')
        
    
