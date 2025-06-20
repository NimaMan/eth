"""
Uniswap V4 Pool Implementation

Architectural Revolution:
- **Singleton PoolManager**: All pools managed by single contract (0x000000000004444C5DC75cB358380d2E3de08a90)
- **PoolId-based**: Pools identified by keccak256 hash of PoolKey, not addresses
- **Hooks System**: Custom logic attachable to pools for advanced features
- **Transient Storage**: Flash accounting eliminates intermediate state
- **Native Multi-hop**: Complex swaps handled natively in single transaction

Pool Key Components:
- Currency0/Currency1: Token pair (deterministically sorted)
- Fee: Dynamic fee tiers (can be hook-controlled)
- TickSpacing: Granularity of liquidity positions
- Hooks: Contract address for custom pool logic

Event Processing:
- univ4_initializes: Pool creation with PoolKey→PoolId mapping
- univ4_swaps: Price changes with PoolId identification
- univ4_modify_liquidities: Position management
- univ4_donates: Fee donations (V4 exclusive feature)
- univ4_hook_events: Custom hook interactions

Blockchain Interface:
- getSlot0(poolId): Current price and tick for specific pool
- getLiquidity(poolId): Active liquidity at current tick
- getPoolKey(poolId): Retrieve PoolKey from PoolId
"""

from typing import Dict, Optional, List, Tuple
from dataclasses import dataclass, field
from collections import deque
import math
from .base_pool import BasePool
from eth_block_processor.data_models.txn_models import ProcessedTransaction


# Uniswap V4 PoolManager contract ABI for blockchain queries
UNISWAP_V4_POOL_MANAGER_ABI = [
    {
        "inputs": [{"internalType": "bytes32", "name": "poolId", "type": "bytes32"}],
        "name": "getSlot0",
        "outputs": [
            {"internalType": "uint160", "name": "sqrtPriceX96", "type": "uint160"},
            {"internalType": "int24", "name": "tick", "type": "int24"},
            {"internalType": "uint8", "name": "protocolFee", "type": "uint8"},
            {"internalType": "uint8", "name": "hookFee", "type": "uint8"}
        ],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [{"internalType": "bytes32", "name": "poolId", "type": "bytes32"}],
        "name": "getLiquidity",
        "outputs": [{"internalType": "uint128", "name": "liquidity", "type": "uint128"}],
        "stateMutability": "view",
        "type": "function"
    }
]


@dataclass
class PoolKey:
    """V4 Pool Key - uniquely identifies a pool in the PoolManager."""
    currency0: str
    currency1: str
    fee: int
    tick_spacing: int
    hooks: str

@dataclass
class V4Tick:
    """V4 Tick state."""
    liquidity_gross: int = 0
    liquidity_net: int = 0


class UniswapV4Pool(BasePool):
    """
    Uniswap V4 Pool, correctly modeling concentrated liquidity within the singleton architecture.
    """
    POOL_MANAGER = "0x000000000004444C5DC75cB358380d2E3de08a90"
    
    def __init__(self, pool_id: str, pool_key: PoolKey, token_address: str, 
                 denom_address: str, token1_is_denom: bool = True):
        super().__init__(self.POOL_MANAGER, token_address, denom_address, token1_is_denom)
        
        self.pool_id = pool_id
        self.pool_key = pool_key
        
        # Display address for compatibility (PoolManager#poolId format)
        self.display_address = f"{self.POOL_MANAGER}#{pool_id}"
        
        self.sqrt_price_x96: int = 0
        self.current_tick: int = 0
        self.current_liquidity: int = 0
        
        self.ticks: Dict[int, V4Tick] = {}

    def get_protocol(self) -> str:
        return "V4"
        
    def process_transaction(self, transaction: ProcessedTransaction):
        # V4 uses ModifyLiquidity for both mints and burns
        for modify in getattr(transaction, 'uniswap_v4_modifies', []):
            if modify.get('pool_id') == self.pool_id:
                self._process_modify_liquidity(modify, transaction)

        for swap in getattr(transaction, 'uniswap_v4_swaps', []):
            if swap.get('pool_id') == self.pool_id:
                self._process_swap(swap, transaction)
    
    def _process_swap(self, swap: dict, transaction: ProcessedTransaction):
        self._mark_trading_enabled(transaction)
        
        self.sqrt_price_x96 = int(swap.get('sqrt_price_x96', 0))
        new_tick = int(swap.get('tick', 0))
        
        if self.current_tick != new_tick:
            self.current_liquidity = self._get_liquidity_for_tick(new_tick)
            self.current_tick = new_tick

        self._update_virtual_reserves()
        self._update_prices()

        amount0 = float(swap.get('amount0', 0))
        amount1 = float(swap.get('amount1', 0))
        self.state.volume0_in += max(0, amount0)
        self.state.volume0_out += max(0, -amount0)
        self.state.volume1_in += max(0, amount1)
        self.state.volume1_out += max(0, -amount1)
        self.state.total_swaps += 1
        
        self.swap_events.append(swap)

    def _process_modify_liquidity(self, modify: dict, transaction: ProcessedTransaction):
        self._mark_trading_enabled(transaction)

        liquidity_delta = int(modify.get('liquidity_delta', 0))
        tick_lower = int(modify.get('tick_lower', 0))
        tick_upper = int(modify.get('tick_upper', 0))

        # Positive delta is a mint, negative is a burn
        if liquidity_delta > 0:
            self.state.total_mints += 1
            self.mint_events.append(modify)
        else:
            self.state.total_burns += 1
            self.burn_events.append(modify)

        self._update_tick(tick_lower, liquidity_delta)
        self._update_tick(tick_upper, -liquidity_delta)

        if tick_lower <= self.current_tick < tick_upper:
            self.current_liquidity += liquidity_delta
            self.current_liquidity = max(0, self.current_liquidity)

        self._update_virtual_reserves()

    def _update_tick(self, tick_index: int, liquidity_delta: int):
        if tick_index not in self.ticks:
            self.ticks[tick_index] = V4Tick()
        
        tick = self.ticks[tick_index]
        tick.liquidity_gross += abs(liquidity_delta)
        tick.liquidity_net += liquidity_delta

    def _get_liquidity_for_tick(self, tick_index: int) -> int:
        active_liquidity = self.current_liquidity
        sorted_ticks = sorted(self.ticks.keys())
        
        start_index = 0
        try:
            start_index = sorted_ticks.index(self.current_tick)
        except ValueError:
            pass
            
        for i in range(start_index, len(sorted_ticks)):
            t = sorted_ticks[i]
            if t > tick_index:
                break
            active_liquidity += self.ticks[t].liquidity_net
            
        return active_liquidity

    def _update_virtual_reserves(self):
        if self.sqrt_price_x96 == 0 or self.current_liquidity == 0:
            self.state.reserve0 = 0
            self.state.reserve1 = 0
            return

        sqrt_price = self.sqrt_price_x96 / (2**96)
        
        reserve0_raw = self.current_liquidity / sqrt_price
        reserve1_raw = self.current_liquidity * sqrt_price
        
        token0_decimals = self._get_token0_decimals()
        token1_decimals = self._get_token1_decimals()

        self.state.reserve0 = reserve0_raw / (10**token0_decimals)
        self.state.reserve1 = reserve1_raw / (10**token1_decimals)
        
    def _update_prices(self):
        if self.state.reserve0 > 0 and self.state.reserve1 > 0:
            self.state.price0 = self.state.reserve1 / self.state.reserve0
            self.state.price1 = self.state.reserve0 / self.state.reserve1
            self.price_history.append((self.state.last_update_block, self.get_price()))

    def _get_token0_decimals(self) -> int:
        # V4 PoolKey currency0/1 is already sorted, but our token/denom is not.
        # We must align them with the token1_is_denom flag.
        pool_key_token0 = self.pool_key.currency0
        if self.token1_is_denom: # Our token is token0 in the pair
            return self.get_token_decimals() if self.token_address == pool_key_token0 else self.get_denom_decimals()
        else: # Our token is token1 in the pair
            return self.get_denom_decimals() if self.denom_address == pool_key_token0 else self.get_token_decimals()

    def _get_token1_decimals(self) -> int:
        pool_key_token1 = self.pool_key.currency1
        if self.token1_is_denom: # Our token is token0 in the pair
            return self.get_denom_decimals() if self.denom_address == pool_key_token1 else self.get_token_decimals()
        else: # Our token is token1 in the pair
            return self.get_token_decimals() if self.token_address == pool_key_token1 else self.get_denom_decimals()
    
    def get_reserves_from_blockchain(self, block_identifier='latest') -> Tuple[float, float, bool]:
        """
        Fetch V4 pool state from blockchain using multiple approaches.
        
        Returns:
            Tuple of (denom_reserve, token_reserve, success)
        """
        try:
            if not hasattr(self, 'w3') or not self.w3.is_connected():
                return 0.0, 0.0, False
                
            # Try multiple approaches to get pool state
            pool_state = self._get_pool_state()
            
            if not pool_state or pool_state.get('sqrtPriceX96', 0) == 0:
                return 0.0, 0.0, False
                
            # Calculate reserves from pool state
            reserve0, reserve1 = self._calculate_reserves_from_state(pool_state)
            
            # Determine which is denom vs token based on configuration
            if self.token1_is_denom:
                # token0 = our token, token1 = denom
                denom_reserve = reserve1
                token_reserve = reserve0
            else:
                # token0 = denom, token1 = our token  
                denom_reserve = reserve0
                token_reserve = reserve1
            
            return denom_reserve, token_reserve, True
            
        except Exception:
            return 0.0, 0.0, False
    
    def _get_pool_state(self) -> Optional[Dict]:
        """Get pool state trying multiple approaches."""
        approaches = [
            self._try_pools_function,
            self._try_getSlot0_function,
            self._try_storage_read
        ]
        
        for approach in approaches:
            try:
                result = approach()
                if result and result.get('sqrtPriceX96', 0) > 0:
                    return result
            except Exception:
                continue
        
        return None
    
    def _try_pools_function(self) -> Optional[Dict]:
        """Try pools() function approach."""
        abi = [{
            "inputs": [{"type": "bytes32"}],
            "name": "pools",
            "outputs": [
                {"type": "uint160", "name": "sqrtPriceX96"},
                {"type": "int24", "name": "tick"},
                {"type": "uint128", "name": "liquidity"}
            ],
            "stateMutability": "view",
            "type": "function"
        }]
        
        contract = self.w3.eth.contract(address=self.POOL_MANAGER, abi=abi)
        pool_id_bytes = bytes.fromhex(self.pool_id[2:] if self.pool_id.startswith('0x') else self.pool_id)
        
        result = contract.functions.pools(pool_id_bytes).call()
        
        return {
            'sqrtPriceX96': result[0],
            'tick': result[1],
            'liquidity': result[2]
        }
    
    def _try_getSlot0_function(self) -> Optional[Dict]:
        """Try getSlot0 + getLiquidity approach."""
        slot0_abi = [{
            "inputs": [{"type": "bytes32"}],
            "name": "getSlot0",
            "outputs": [{"type": "uint160"}, {"type": "int24"}],
            "stateMutability": "view",
            "type": "function"
        }]
        
        liquidity_abi = [{
            "inputs": [{"type": "bytes32"}],
            "name": "getLiquidity", 
            "outputs": [{"type": "uint128"}],
            "stateMutability": "view",
            "type": "function"
        }]
        
        pool_id_bytes = bytes.fromhex(self.pool_id[2:] if self.pool_id.startswith('0x') else self.pool_id)
        
        slot0_contract = self.w3.eth.contract(address=self.POOL_MANAGER, abi=slot0_abi)
        liquidity_contract = self.w3.eth.contract(address=self.POOL_MANAGER, abi=liquidity_abi)
        
        slot0_result = slot0_contract.functions.getSlot0(pool_id_bytes).call()
        liquidity = liquidity_contract.functions.getLiquidity(pool_id_bytes).call()
        
        return {
            'sqrtPriceX96': slot0_result[0],
            'tick': slot0_result[1],
            'liquidity': liquidity
        }
    
    def _try_storage_read(self) -> Optional[Dict]:
        """Try direct storage reading."""
        from eth_utils import keccak
        
        pool_id_bytes = bytes.fromhex(self.pool_id[2:] if self.pool_id.startswith('0x') else self.pool_id)
        
        # Calculate storage key for pools mapping (slot 0)
        storage_key = keccak(pool_id_bytes + (0).to_bytes(32, 'big'))
        
        # Read slot0
        slot0_data = self.w3.eth.get_storage_at(self.POOL_MANAGER, storage_key)
        
        # Read liquidity (next slot)
        liquidity_slot = int.from_bytes(storage_key, 'big') + 1
        liquidity_data = self.w3.eth.get_storage_at(self.POOL_MANAGER, liquidity_slot.to_bytes(32, 'big'))
        
        # Decode packed slot0
        slot0_int = int.from_bytes(slot0_data, 'big')
        
        # Extract sqrtPriceX96 (160 bits) and tick (24 bits)
        sqrtPriceX96 = slot0_int & ((1 << 160) - 1)
        tick = (slot0_int >> 160) & ((1 << 24) - 1)
        
        # Handle negative ticks
        if tick > 0x7FFFFF:
            tick = tick - 0x1000000
            
        liquidity = int.from_bytes(liquidity_data, 'big')
        
        return {
            'sqrtPriceX96': sqrtPriceX96,
            'tick': tick,
            'liquidity': liquidity
        }
    
    def _calculate_reserves_from_state(self, pool_state: Dict) -> Tuple[float, float]:
        """Calculate virtual reserves from pool state."""
        sqrt_price_x96 = pool_state['sqrtPriceX96']
        liquidity = pool_state['liquidity']
        
        if sqrt_price_x96 == 0 or liquidity == 0:
            return 0.0, 0.0
            
        sqrt_price = sqrt_price_x96 / (2**96)
        
        # Calculate raw reserves
        reserve0_raw = float(liquidity) / sqrt_price
        reserve1_raw = float(liquidity) * sqrt_price
        
        # Apply decimals
        token0_decimals = self._get_token0_decimals()
        token1_decimals = self._get_token1_decimals()
        
        reserve0 = reserve0_raw / (10 ** token0_decimals)
        reserve1 = reserve1_raw / (10 ** token1_decimals)
        
        return reserve0, reserve1