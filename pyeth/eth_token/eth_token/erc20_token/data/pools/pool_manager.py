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

from typing import Dict, List, Optional, Tuple
from collections import defaultdict

from .base_pool import BasePool
from .uniswap_v2_pool import UniswapV2Pool, UNISWAP_V2_PAIR_ABI
from .uniswap_v3_pool import UniswapV3Pool
from .uniswap_v4_pool import UniswapV4Pool, PoolKey
from eth_block_processor.chain_utils.pool_addresses import POOL_FACTORIES
from eth_block_processor.data_models.txn_models import ProcessedTransaction


class PoolManager:
    """
    Manages all pools for a token.
    
    Routes transaction events to the appropriate pool instances
    and provides aggregated views across all pools.
    """
    
    def __init__(self, token_address: str, logger):
        from web3 import Web3
        self.token_address = Web3.to_checksum_address(token_address)
        
        # Pool storage by address
        self.pools: Dict[str, BasePool] = {}
        
        # Indexes for quick lookup
        self.pools_by_protocol: Dict[str, List[str]] = defaultdict(list)
        self.pools_by_denom: Dict[str, List[str]] = defaultdict(list)
        
        # V4 pools tracked separately by PoolId
        self.v4_pools: Dict[str, UniswapV4Pool] = {}  # poolId -> V4Pool    
        self.v4_pool_manager = '0x000000000004444c5dc75cb358380d2e3de08a90'
        
        # Trading enabled tracking
        self.trading_enabled = False
        self.trading_enabled_block = 0
        self.trading_enabled_txn = ""
    
        self.logger = logger
        
    def process_transaction(self, transaction: ProcessedTransaction):
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
            pool.process_transaction(transaction)
            
        # Route V4 events to V4 pools
        for v4_pool in self.v4_pools.values():
            v4_pool.process_transaction(transaction)
            
        # Check for trading enabled after processing all pools
        self._check_trading_enabled(transaction)
            
    def _check_pool_creations(self, transaction: ProcessedTransaction):
        """Check for new pool creation events."""
        # V2 pair creation
        for pair_event in getattr(transaction, 'pair_events', []):
            self._handle_v2_creation(pair_event, transaction)
            
        # V3 pool creation
        for pool_event in getattr(transaction, 'uniswap_v3_pools', []):
            self._handle_v3_creation(pool_event, transaction)
            
        # V4 pool initialization (different pattern - uses PoolId)
        for init_event in getattr(transaction, 'uniswap_v4_initializes', []):
            self._handle_v4_initialization(init_event, transaction)
        
    def _handle_v2_creation(self, pair_event: dict, transaction: ProcessedTransaction):
        """Handle V2 pair creation."""
        from web3 import Web3
        
        pair_address = Web3.to_checksum_address(pair_event['pair_address'])
        token0 = Web3.to_checksum_address(pair_event['token0'])
        token1 = Web3.to_checksum_address(pair_event['token1'])
        
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
        pool = UniswapV2Pool(
            pool_address=pair_address,
            token_address=self.token_address,
            denom_address=denom_address,
            token1_is_denom=token1_is_denom
        )
        
        pool.creation_block = transaction.block_number
        pool.creation_txn = transaction.hash
        self._register_pool(pool)# Register pool
        
    def _handle_v3_creation(self, pool_event: dict, transaction: ProcessedTransaction):
        """Handle V3 pool creation."""
        from web3 import Web3
        
        # V3 pool creation events use 'pool' field for address
        pool_address = Web3.to_checksum_address(pool_event.get('pool', pool_event.get('pool_address', '')))
        token0 = Web3.to_checksum_address(pool_event['token0'])
        token1 = Web3.to_checksum_address(pool_event['token1'])
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
        pool = UniswapV3Pool(
            pool_address=pool_address,
            token_address=self.token_address,
            denom_address=denom_address,
            token1_is_denom=token1_is_denom,
            fee_tier=fee
        )
        
        pool.creation_block = transaction.block_number
        pool.creation_txn = transaction.hash
        self._register_pool(pool)# Register pool
        
    def _handle_v4_initialization(self, init_event: dict, transaction: ProcessedTransaction):
        """Handle V4 pool initialization.
        
        V4 pools are identified by PoolId, not address.
        All V4 pools share the same PoolManager address.
        """
        from web3 import Web3
        
        # V4 events use event_id for pool identification
        pool_id = init_event.get('event_id', init_event.get('pool_id', ''))
        currency0 = Web3.to_checksum_address(init_event['currency0'])
        currency1 = Web3.to_checksum_address(init_event['currency1'])
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
        pool = UniswapV4Pool(
            pool_id=pool_id,
            pool_key=pool_key,
            token_address=self.token_address,
            denom_address=denom_address,
            token1_is_denom=token1_is_denom
        )
        
        pool.creation_block = transaction.block_number
        pool.creation_txn = transaction.hash
        self._register_v4_pool(pool)# Register V4 pool
        
    def _register_pool(self, pool: BasePool):
        """Register a pool in the manager."""
        self.pools[pool.pool_address] = pool
        self.pools_by_protocol[pool.get_protocol()].append(pool.pool_address)
        self.pools_by_denom[pool.denom_address].append(pool.pool_address)
        
    def _register_v4_pool(self, pool: UniswapV4Pool):
        """Register a V4 pool in the manager.
        
        V4 pools are tracked separately by PoolId.
        """
        self.v4_pools[pool.pool_id] = pool
        self.pools_by_protocol['V4'].append(pool.pool_id)
        self.pools_by_denom[pool.denom_address].append(pool.pool_id)
        
    def add_pool(self, pool_address: str, protocol: str, denom_address: str, 
                 token1_is_denom: bool = True, **kwargs):
        """
        Manually add a pool.
        
        Used for pools that already exist when we start tracking a token.
        """
        # Convert to checksum addresses first
        from web3 import Web3
        pool_address = Web3.to_checksum_address(pool_address)
        denom_address = Web3.to_checksum_address(denom_address)
        
        # Skip if already exists
        if pool_address in self.pools:
            return self.pools[pool_address]# Return pool if already exists
            
        # Create appropriate pool instance
        if protocol == "V2":
            pool = UniswapV2Pool(
                pool_address=pool_address,
                token_address=self.token_address,
                denom_address=denom_address,
                token1_is_denom=token1_is_denom
            )
        elif protocol == "V3":
            pool = UniswapV3Pool(
                pool_address=pool_address,
                token_address=self.token_address,
                denom_address=denom_address,
                token1_is_denom=token1_is_denom,
                fee_tier=kwargs.get('fee_tier', 3000)
            )
        elif protocol == "V4":
            # V4 pools should be created through initialization events, not manual addition
            # This is a placeholder that won't work properly
            self.logger.warning(f"V4 pools should be created through initialization events, not manual addition")
            return None
        else:
            self.logger.error(f"Unknown protocol {protocol} for pool {pool_address} and token {self.token_address}")
            return None
            
        self._register_pool(pool)# Register pool
        return pool
        
    def get_pool(self, pool_address: str) -> Optional[BasePool]:
        """Get a specific pool by address or display address."""
        from web3 import Web3
        
        # Check if it's a V4 display address (PoolManager#poolId)
        if '#' in pool_address:
            parts = pool_address.split('#')
            if len(parts) == 2:
                pool_id = parts[1]
                return self.v4_pools.get(pool_id)
        
        # Try regular pools with checksum conversion
        try:
            pool = self.pools.get(Web3.to_checksum_address(pool_address))
            if pool:
                return pool
        except ValueError:
            # If checksum conversion fails, try as-is
            pool = self.pools.get(pool_address)
            if pool:
                return pool
                
        return None
        
    def get_pools_by_protocol(self, protocol: str) -> List[BasePool]:
        """Get all pools for a specific protocol."""
        addresses = self.pools_by_protocol.get(protocol, [])
        return [self.pools[addr] for addr in addresses]
        
    def get_pools_by_denom(self, denom_address: str) -> List[BasePool]:
        """Get all pools paired with a specific denomination token."""
        from web3 import Web3
        addresses = self.pools_by_denom.get(Web3.to_checksum_address(denom_address), [])
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
        
    def get_stats(self) -> Dict:
        """Get aggregated statistics across all pools."""
        stats = {
            'total_pools': len(self.pools),
            'pools_by_protocol': {
                protocol: len(addresses) 
                for protocol, addresses in self.pools_by_protocol.items()
            },
            'healthy_pools': sum(1 for p in self.pools.values() if p.is_healthy()),
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
            denom_symbol = self._get_token_symbol(pool.denom_address)
            pool_info[address] = {
                'pool_type': pool.get_protocol().upper(),  # "V2", "V3"
                'denom_address': pool.denom_address,
                'denom_currency': denom_symbol,
                'decimals': 18,  # Default, should get from token contract
                'token1_is_denom': pool.token1_is_denom
            }
            
        # Add V4 pools (use display address for compatibility)
        for pool_id, pool in self.v4_pools.items():
            pool_info[pool.display_address] = {
                'pool_type': 'V4',
                'denom_address': pool.denom_address,
                'denom_currency': self._get_token_symbol(pool.denom_address),
                'decimals': 18,
                'token1_is_denom': pool.token1_is_denom,
                'pool_id': pool_id  # Extra field for V4
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
            currencies[address] = self._get_token_symbol(pool.denom_address)
            
        for pool in self.v4_pools.values():
            currencies[pool.display_address] = self._get_token_symbol(pool.denom_address)
            
        return currencies
        
    def has_pools(self) -> bool:
        """Check if token has any pools."""
        return len(self.pools) > 0 or len(self.v4_pools) > 0
    
    def has_v2_pools(self) -> bool:
        """Check if any V2 pools exist."""
        return len(self.pools_by_protocol.get('V2', [])) > 0
    
    def has_v3_pools(self) -> bool:
        """Check if any V3 pools exist."""
        return len(self.pools_by_protocol.get('V3', [])) > 0
    
    def has_v4_pools(self) -> bool:
        """Check if any V4 pools exist."""
        return len(self.v4_pools) > 0
    
    def compute_v2_pool_address(self, denom_address: str) -> str:
        """
        Compute deterministic V2 pool address.
        
        Pool addresses in V2 are deterministic based on token pair.
        """
        from eth_utils import keccak
        
        # Sort tokens
        token0, token1 = sorted([self.token_address, denom_address])
        
        # V2 uses CREATE2 with salt = keccak(token0, token1)
        salt = keccak(bytes.fromhex(token0[2:]) + bytes.fromhex(token1[2:]))
        
        # This is a simplified version - actual implementation would use CREATE2
        return f"0x{salt.hex()[:40]}"
    
    def compute_v3_pool_address(self, denom_address: str, fee: int = 3000) -> str:
        """
        Compute deterministic V3 pool address.
        
        Pool addresses in V3 are deterministic based on token pair and fee.
        """
        from eth_utils import keccak
        
        # Sort tokens
        token0, token1 = sorted([self.token_address, denom_address])
        
        # V3 includes fee in the salt
        fee_bytes = fee.to_bytes(3, 'big')
        salt_data = bytes.fromhex(token0[2:]) + bytes.fromhex(token1[2:]) + fee_bytes
        salt = keccak(salt_data)
        
        return f"0x{salt.hex()[:40]}"
    
    def get_or_create_v2_pool(self, denom_address: str) -> Optional[BasePool]:
        """
        Get existing V2 pool or compute its address if it should exist.
        """
        pool_address = self.compute_v2_pool_address(denom_address)
        
        if pool_address in self.pools:
            return self.pools[pool_address]
        
        # Check if pool exists on-chain and create if so
        # For now, return None - would need to call factory.getPair()
        return None
    
    def get_or_create_v3_pool(self, denom_address: str, fee: int = 3000) -> Optional[BasePool]:
        """
        Get existing V3 pool or compute its address if it should exist.
        """
        pool_address = self.compute_v3_pool_address(denom_address, fee)
        
        if pool_address in self.pools:
            return self.pools[pool_address]
        
        # Check if pool exists on-chain and create if so
        # For now, return None - would need to call factory.getPool()
        return None
    
    def get_total_lp_supply(self) -> Dict[str, float]:
        """Get total LP supply for all V2 pools.
        
        Returns:
            Dict mapping pool address to LP token total supply
        """
        lp_supplies = {}
        for address, pool in self.pools.items():
            if pool.get_protocol() == 'V2' and hasattr(pool, 'lp_total_supply'):
                lp_supplies[address] = pool.lp_total_supply
        return lp_supplies
    
    def get_lp_holder_stats(self) -> Dict:
        """Get LP holder statistics across all pools.
        
        Returns:
            Dictionary with LP holder statistics:
            - total_lp_holders: Unique LP holders across all pools
            - pools_with_lp: Number of V2 pools
            - largest_lp_pool: Pool with highest LP supply
            - most_holders_pool: Pool with most LP holders
            - top_lp_providers: Top LP providers across all pools
        """
        stats = {
            'total_lp_holders': 0,
            'pools_with_lp': 0,
            'largest_lp_pool': None,
            'most_holders_pool': None,
            'top_lp_providers': []
        }
        
        unique_holders = set()
        max_supply = 0
        max_holders = 0
        all_provider_balances = defaultdict(float)  # Aggregate LP holdings across pools
        
        for address, pool in self.pools.items():
            if pool.get_protocol() == 'V2' and hasattr(pool, 'lp_holders'):
                stats['pools_with_lp'] += 1
                
                # Track unique holders
                pool_holders = {addr for addr, balance in pool.lp_holders.items() if balance > 0}
                unique_holders.update(pool_holders)
                
                # Find pool with highest LP supply
                if hasattr(pool, 'lp_total_supply') and pool.lp_total_supply > max_supply:
                    max_supply = pool.lp_total_supply
                    stats['largest_lp_pool'] = address
                    
                # Find pool with most holders
                if len(pool_holders) > max_holders:
                    max_holders = len(pool_holders)
                    stats['most_holders_pool'] = address
                    
                # Aggregate LP holdings across all pools
                for holder, balance in pool.lp_holders.items():
                    if balance > 0:
                        all_provider_balances[holder] += balance
        
        stats['total_lp_holders'] = len(unique_holders)
        
        # Get top LP providers across all pools
        if all_provider_balances:
            sorted_providers = sorted(
                all_provider_balances.items(),
                key=lambda x: x[1],
                reverse=True
            )[:10]  # Top 10
            
            stats['top_lp_providers'] = [
                {
                    'address': addr,
                    'total_lp_balance': balance,
                    'pools_count': sum(
                        1 for p in self.pools.values() 
                        if p.get_protocol() == 'V2' and 
                        hasattr(p, 'lp_holders') and 
                        addr in p.lp_holders and 
                        p.lp_holders[addr] > 0
                    )
                }
                for addr, balance in sorted_providers
            ]
        
        return stats
    
    def get_pool_ownership_distribution(self, pool_address: str) -> Dict:
        """Get ownership distribution for a specific pool.
        
        Args:
            pool_address: The pool address to analyze
            
        Returns:
            Dictionary with ownership distribution data
        """
        pool = self.get_pool(pool_address)
        if not pool or pool.get_protocol() != 'V2':
            return {}
            
        if not hasattr(pool, 'get_top_lp_holders'):
            return {}
            
        # Get top holders
        top_holders = pool.get_top_lp_holders(20)
        
        # Calculate concentration metrics
        total_top_20_share = sum(share for _, _, share in top_holders)
        
        return {
            'pool_address': pool_address,
            'lp_total_supply': getattr(pool, 'lp_total_supply', 0),
            'holder_count': len([h for h, b in getattr(pool, 'lp_holders', {}).items() if b > 0]),
            'top_holder': top_holders[0] if top_holders else None,
            'top_5_concentration': sum(share for _, _, share in top_holders[:5]),
            'top_10_concentration': sum(share for _, _, share in top_holders[:10]),
            'top_20_concentration': total_top_20_share,
            'top_holders': [
                {
                    'address': addr,
                    'balance': balance,
                    'share_percent': share
                }
                for addr, balance, share in top_holders[:10]
            ]
        }
    
    def _check_swap_events_for_pools(self, transaction: ProcessedTransaction):
        """
        Check swap events for pools we haven't seen yet.
        
        This is useful when:
        1. We start monitoring a token that already has pools
        2. Pool creation events are missing from transactions
        3. We want to discover pools from trading activity
        """
        # Check V2 swaps
        for swap in getattr(transaction, 'uniswap_v2_swaps', []):
            pair_address = swap.get('pair_address', '')
            if pair_address:
                from web3 import Web3
                pair_address = Web3.to_checksum_address(pair_address)
                if pair_address not in self.pools:
                    self._try_create_v2_pool_from_swap(pair_address, transaction)
                
        # Check V3 swaps  
        for swap in getattr(transaction, 'uniswap_v3_swaps', []):
            pool_address = swap.get('pool_address', '')
            if pool_address:
                from web3 import Web3
                pool_address = Web3.to_checksum_address(pool_address)
                if pool_address not in self.pools:
                    self._try_create_v3_pool_from_swap(pool_address, transaction)
                
    def _try_create_v2_pool_from_swap(self, pair_address: str, transaction: ProcessedTransaction):
        """
        Try to create a V2 pool from a swap event by querying the blockchain.
        """
        try:
            # Skip if already being processed
            if hasattr(self, '_processing_pools'):
                if pair_address in self._processing_pools:
                    return
            else:
                self._processing_pools = set()
                
            self._processing_pools.add(pair_address)
            
            
            # Get Web3 instance (assuming available via pool instances)
            if not hasattr(self, 'w3'):
                from web3 import Web3
                import os
                eth_node_url = os.environ.get("ETH_NODE_URL", "http://127.0.0.1:8545")
                self.w3 = Web3(Web3.HTTPProvider(eth_node_url))
                
            # Get pool contract
            pair_contract = self.w3.eth.contract(
                address=self.w3.to_checksum_address(pair_address),
                abi=UNISWAP_V2_PAIR_ABI
            )
            
            # Get token addresses
            token0 = self.w3.to_checksum_address(pair_contract.functions.token0().call())
            token1 = self.w3.to_checksum_address(pair_contract.functions.token1().call())
            
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
                
            # Create pool instance
            pool = UniswapV2Pool(
                pool_address=pair_address,
                token_address=self.token_address,
                denom_address=denom_address,
                token1_is_denom=token1_is_denom
            )
            
            pool.creation_block = transaction.block_number
            pool.creation_txn = transaction.hash
            pool.detected_from_swap = True
            
            # Register pool
            self._register_pool(pool)
            
            
        except Exception as e:
            self.logger.debug(f"Failed to create V2 pool from swap {pair_address}: {e}")
        finally:
            self._processing_pools.discard(pair_address)
            
    def _try_create_v3_pool_from_swap(self, pool_address: str, transaction: ProcessedTransaction):
        """
        Try to create a V3 pool from a swap event by querying the blockchain.
        """
        try:
            # Skip if already being processed
            if hasattr(self, '_processing_pools'):
                if pool_address in self._processing_pools:
                    return
            else:
                self._processing_pools = set()
                
            self._processing_pools.add(pool_address)
            
            # Import V3 pool class
            from .uniswap_v3_pool import UniswapV3Pool
            
            # V3 pool ABI (minimal)
            V3_POOL_ABI = [
                {
                    "constant": True,
                    "inputs": [],
                    "name": "token0",
                    "outputs": [{"internalType": "address", "name": "", "type": "address"}],
                    "payable": False,
                    "stateMutability": "view",
                    "type": "function"
                },
                {
                    "constant": True,
                    "inputs": [],
                    "name": "token1",
                    "outputs": [{"internalType": "address", "name": "", "type": "address"}],
                    "payable": False,
                    "stateMutability": "view",
                    "type": "function"
                },
                {
                    "constant": True,
                    "inputs": [],
                    "name": "fee",
                    "outputs": [{"internalType": "uint24", "name": "", "type": "uint24"}],
                    "payable": False,
                    "stateMutability": "view",
                    "type": "function"
                }
            ]
            
            # Get Web3 instance
            if not hasattr(self, 'w3'):
                from web3 import Web3
                import os
                eth_node_url = os.environ.get("ETH_NODE_URL", "http://127.0.0.1:8545")
                self.w3 = Web3(Web3.HTTPProvider(eth_node_url))
                
            # Get pool contract
            pool_contract = self.w3.eth.contract(
                address=self.w3.to_checksum_address(pool_address),
                abi=V3_POOL_ABI
            )
            
            # Get pool details
            token0 = self.w3.to_checksum_address(pool_contract.functions.token0().call())
            token1 = self.w3.to_checksum_address(pool_contract.functions.token1().call())
            fee = pool_contract.functions.fee().call()
            
            # Check if our token is involved
            if token0 == self.token_address:
                denom_address = token1
                token1_is_denom = True
            elif token1 == self.token_address:
                denom_address = token0
                token1_is_denom = False
            else:
                # Our token not in this pool
                return
                
            # Create pool instance
            pool = UniswapV3Pool(
                pool_address=pool_address,
                token_address=self.token_address,
                denom_address=denom_address,
                token1_is_denom=token1_is_denom,
                fee_tier=fee
            )
            
            pool.creation_block = transaction.block_number
            pool.creation_txn = transaction.hash
            pool.detected_from_swap = True
            
            # Register pool
            self._register_pool(pool)
            
            self.logger.info(
                f"Detected V3 pool {pool_address[:8]}... from swap event "
                f"(paired with {self._get_token_symbol(denom_address)}, fee: {fee/10000}%)"
            )
            
        except Exception as e:
            self.logger.debug(f"Failed to create V3 pool from swap {pool_address}: {e}")
        finally:
            self._processing_pools.discard(pool_address)
    
    def _get_token_symbol(self, token_address: str) -> str:
        """Get token symbol helper.
        
        For now returns a simple mapping. Should be enhanced to 
        query from blockchain or database.
        """
        # Common token mappings
        common_tokens = {
            '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2': 'WETH',
            '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48': 'USDC',
            '0xdac17f958d2ee523a2206206994597c13d831ec7': 'USDT',
            '0x6b175474e89094c44da98b954eedeac495271d0f': 'DAI',
        }
        
        return common_tokens.get(token_address.lower(), 'Unknown')
    
    def _check_trading_enabled(self, transaction: ProcessedTransaction):
        """
        Check if trading is enabled on any pool.
        
        Trading is enabled when we see any swap, mint, or burn events on any pool.
        """
        if self.trading_enabled:
            return  # Already enabled
            
        # Check for any activity that indicates trading is enabled
        has_activity = False
        
        # Check V2 swaps
        if getattr(transaction, 'uniswap_v2_swaps', []):
            has_activity = True
            
        # Check V3 swaps
        if getattr(transaction, 'uniswap_v3_swaps', []):
            has_activity = True
            
        # Check V4 swaps
        if getattr(transaction, 'uniswap_v4_swaps', []):
            has_activity = True
            
        # Check mints/burns (liquidity operations)
        if (getattr(transaction, 'uniswap_v3_mints', []) or 
            getattr(transaction, 'uniswap_v3_burns', []) or
            getattr(transaction, 'uniswap_v4_modifies', [])):
            has_activity = True
        
        if has_activity:
            self.trading_enabled = True
            self.trading_enabled_block = transaction.block_number
            self.trading_enabled_txn = transaction.hash
            self.logger.info(f"Trading enabled detected at block {transaction.block_number}, tx {transaction.hash}")
    
    def is_trading_enabled(self) -> bool:
        """Check if trading is enabled on any pool."""
        return self.trading_enabled
    
    def get_trading_enabled_info(self):
        """Get trading enabled information."""
        return {
            'enabled': self.trading_enabled,
            'block': self.trading_enabled_block,
            'txn': self.trading_enabled_txn
        }

