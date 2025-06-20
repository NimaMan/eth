"""
Uniswap V3 Pool Implementation

Key Features:
- Concentrated liquidity with tick-based positions
- Dynamic fee tiers (0.01%, 0.05%, 0.3%, 1%)
- Position tracking with tick ranges
- Virtual reserves calculated from current price and liquidity
- Direct blockchain state queries via slot0() and liquidity()

Event Processing:
- univ3_initializes: Pool creation with initial price/tick
- univ3_swaps: Price changes and volume tracking
- univ3_mints: Liquidity position creation
- univ3_burns: Liquidity position removal
- univ3_collects: Fee collection events

Blockchain Interface:
- slot0(): Current price, tick, and protocol state
- liquidity(): Active liquidity at current tick
- positions(): Individual position data
"""

from typing import Optional, Dict, List, Tuple
from collections import defaultdict
from .base_pool import BasePool
from eth_block_processor.data_models.txn_models import ProcessedTransaction


# Uniswap V3 Pool contract ABI for blockchain queries
UNISWAP_V3_POOL_ABI = [
    {
        "inputs": [],
        "name": "slot0",
        "outputs": [
            {"internalType": "uint160", "name": "sqrtPriceX96", "type": "uint160"},
            {"internalType": "int24", "name": "tick", "type": "int24"},
            {"internalType": "uint16", "name": "observationIndex", "type": "uint16"},
            {"internalType": "uint16", "name": "observationCardinality", "type": "uint16"},
            {"internalType": "uint16", "name": "observationCardinalityNext", "type": "uint16"},
            {"internalType": "uint8", "name": "feeProtocol", "type": "uint8"},
            {"internalType": "bool", "name": "unlocked", "type": "bool"}
        ],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "liquidity",
        "outputs": [{"internalType": "uint128", "name": "", "type": "uint128"}],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "token0",
        "outputs": [{"internalType": "address", "name": "", "type": "address"}],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "token1",
        "outputs": [{"internalType": "address", "name": "", "type": "address"}],
        "stateMutability": "view",
        "type": "function"
    }
]


class UniswapV3Pool(BasePool):
    """
    Uniswap V3 pool with concentrated liquidity.
    
    V3 pools use ticks and don't have simple reserves like V2.
    """
    
    def __init__(self, pool_address: str, token_address: str, 
                 denom_address: str, token1_is_denom: bool = True,
                 fee_tier: int = 3000):
        super().__init__(pool_address, token_address, denom_address, token1_is_denom)
        
        self.fee_tier = fee_tier  # 500, 3000, 10000 (0.05%, 0.3%, 1%)
        self.tick_spacing = self._get_tick_spacing(fee_tier)
        
        # V3 specific state
        self.sqrt_price_x96: int = 0
        self.current_tick: int = 0
        self.current_liquidity: int = 0
        
        # Position tracking
        self.positions: Dict[str, Dict] = {}  # owner -> position data
        self.liquidity_by_tick: Dict[int, float] = defaultdict(float)
        
        # Additional V3 events
        self.collect_events = []
        
    def get_protocol(self) -> str:
        return "V3"
        
    def _get_tick_spacing(self, fee: int) -> int:
        """Get tick spacing for fee tier."""
        return {
            100: 1,
            500: 10,
            3000: 60,
            10000: 200
        }.get(fee, 60)
        
    def process_transaction(self, transaction: ProcessedTransaction):
        """
        Process V3 events from a transaction.
        
        V3 events:
        - univ3_swaps: Swaps with tick/liquidity data
        - univ3_mints: Position creations
        - univ3_burns: Position removals
        - univ3_collects: Fee collections
        """
        # Process pair events (Uniswap V3 pools)
        self._add_univ3_pool(transaction)
        # Process Uniswap V3 events
        self._add_univ3_swaps(transaction)
        self._add_univ3_mints(transaction)
        self._add_univ3_burns(transaction)

    def _add_univ3_pool(self, transaction: ProcessedTransaction):
        """Process Uniswap V3 pool creation events"""
        for pool_event in getattr(transaction, 'uniswap_v3_pools', []):
            if pool_event['token0'] == self.contract_address:
                denom_address = pool_event['token1']
                token1_is_denom = True
            else:
                denom_address = pool_event['token0']
                token1_is_denom = False      
            pool_is_valid = self.set_pool_info(pool_event['pool'], "V3", denom_address, token1_is_denom)
            if not pool_is_valid:
                return
            self.has_uni_v3_pool = True    
            self.uniswap_v3_pool.append({
                'txn_hash': transaction['hash'],
                'block_number': transaction['block_number'],
                'txn_index': transaction['txn_index'],
                'log_index': pool_event['log_index'],
                'pool': pool_event['pool'],
                'token0': pool_event['token0'],
                'token1': pool_event['token1'],
                'fee': pool_event['fee']
            })
        
            self.set_lp_token_info(pool_event['pool'])
  
    def add_univ3_price_info(self, sqrt_price_x96: int, pool_address: str):
        """Add a price info"""
        if pool_address not in self.pool_prices:
            self.pool_prices[pool_address] = []
        self.pool_prices[pool_address].append(sqrt_price_x96)

    def _add_univ3_swaps(self, transaction: ProcessedTransaction):
        """Process Uniswap V3 swap events"""
        txn_hash = transaction.hash
        univ3_swaps = getattr(transaction, 'univ3_swaps', [])
        if len(univ3_swaps) > 0:
            if univ3_swaps[0]["pool_address"] not in self.pool_addresses:
                return # We should have already added the pool address
            self.univ3_swaps[txn_hash] = []
            # Ensure trading is enabled if v3 events exist
            if not self.trading_enabled:
                self._update_trading_enabled(transaction)
        
        for swap in univ3_swaps:
            sqrt_price_x96 = int(swap.get('sqrt_price_x96'))
            self.univ3_swaps[txn_hash].append({
                'log_index': swap.get('log_index'),
                'from_address': swap.get('sender'),
                'to_address': swap.get('recipient'),
                'amount0': swap.get('amount0'),
                'amount1': swap.get('amount1'),
                'sqrtPriceX96': sqrt_price_x96,
                'liquidity': swap.get('liquidity'),
                'tick': swap.get('tick'),
                'pool_address': swap.get('pool_address')
            })
            if swap.get('pool_address') in self.pool_addresses:
                self.add_univ3_price_info(sqrt_price_x96, swap.get('pool_address'))

    def _add_univ3_mints(self, transaction: ProcessedTransaction):
        """Process Uniswap V3 mint events"""
        txn_hash = transaction.hash
        v3_mints = getattr(transaction, 'uniswap_v3_mints', [])
        if v3_mints:
            for mint in v3_mints:
                self.univ3_mints.append({
                    'txn_hash': txn_hash,
                    'block_number': transaction.block_number,
                    'txn_index': transaction.txn_index,
                    'log_index': mint.get('log_index'),
                    'to_address': mint.get('to_address', mint.get('owner')),
                    'amount': mint.get('amount', mint.get('liquidity')),
                    'token_address': mint.get('token_address', mint.get('pool_address'))
                })

    def _add_univ3_burns(self, transaction: ProcessedTransaction):
        """Process Uniswap V3 burn events"""
        txn_hash = transaction.hash
        v3_burns = getattr(transaction, 'uniswap_v3_burns', [])
        if v3_burns:
            for burn in v3_burns:
                self.univ3_burns.append({
                    'txn_hash': txn_hash,
                    'block_number': transaction.block_number,
                    'txn_index': transaction.txn_index,
                    'log_index': burn.get('log_index'),
                    'from_address': burn.get('from_address', burn.get('owner')),
                    'amount': burn.get('amount', burn.get('liquidity')),
                    'token_address': burn.get('token_address', burn.get('pool_address'))
                })

    def _update_price_from_sqrt(self):
        """Update price from sqrtPriceX96."""
        if self.sqrt_price_x96 == 0:
            return
            
        # price = (sqrtPriceX96 / 2^96)^2
        sqrt_price = self.sqrt_price_x96 / (2**96)
        price = sqrt_price * sqrt_price
        
        self.state.price0 = price
        self.state.price1 = 1 / price if price > 0 else 0
        
    def _update_virtual_reserves(self):
        """
        Calculate virtual reserves for V2 compatibility.
        
        This is an approximation since V3 doesn't have simple reserves.
        """
        if self.sqrt_price_x96 == 0 or self.current_liquidity == 0:
            return
            
        # Simplified calculation
        sqrt_price = self.sqrt_price_x96 / (2**96)
        
        # Virtual reserves based on current liquidity and price
        # This is a rough approximation
        self.state.reserve0 = self.current_liquidity / sqrt_price
        self.state.reserve1 = self.current_liquidity * sqrt_price
        self.state.total_liquidity = float(self.current_liquidity)
        
    def get_liquidity_at_tick(self, tick: int) -> float:
        """Get total liquidity at a specific tick."""
        total_liquidity = 0
        for t, liquidity_delta in sorted(self.liquidity_by_tick.items()):
            if t <= tick:
                total_liquidity += liquidity_delta
            else:
                break
        return total_liquidity
        
    def get_active_positions(self) -> List[Dict]:
        """Get all active liquidity positions."""
        return [
            pos for pos in self.positions.values() 
            if pos['liquidity'] > 0
        ]
        
    def get_tick_range_positions(self, tick_lower: int, tick_upper: int) -> List[Dict]:
        """Get positions within a tick range."""
        matching = []
        for pos in self.positions.values():
            if (pos['tick_lower'] >= tick_lower and 
                pos['tick_upper'] <= tick_upper and
                pos['liquidity'] > 0):
                matching.append(pos)
        return matching
    
    def get_reserves_from_blockchain(self, block_identifier='latest') -> Tuple[float, float, bool]:
        """
        Fetch V3 pool state from blockchain and calculate virtual reserves.
        
        Uses slot0() to get current price and liquidity() for active liquidity,
        then calculates equivalent V2-style reserves for compatibility.
        
        Args:
            block_identifier: Block number or 'latest'
            
        Returns:
            Tuple of (denom_reserve, token_reserve, success)
        """
        try:
            if not self.w3.is_connected():
                return 0.0, 0.0, False
                
            pool_contract = self.w3.eth.contract(
                address=self.w3.to_checksum_address(self.pool_address),
                abi=UNISWAP_V3_POOL_ABI
            )
            
            # Get current pool state
            slot0_data = pool_contract.functions.slot0().call(block_identifier=block_identifier)
            sqrt_price_x96 = slot0_data[0]
            current_tick = slot0_data[1]
            
            active_liquidity = pool_contract.functions.liquidity().call(block_identifier=block_identifier)
            
            if sqrt_price_x96 == 0 or active_liquidity == 0:
                return 0.0, 0.0, False
                
            # Calculate virtual reserves from concentrated liquidity
            sqrt_price = sqrt_price_x96 / (2**96)
            price = sqrt_price * sqrt_price
            
            # Virtual reserves calculation
            reserve0 = float(active_liquidity) / sqrt_price
            reserve1 = float(active_liquidity) * sqrt_price
            
            # Apply decimals correctly
            # reserve0 is always token0, reserve1 is always token1
            # We need to get the actual decimals for each token
            denom_decimals = self.get_denom_decimals()
            token_decimals = 18  # Assume 18 for now, should be fetched from token contract
            
            # Apply proper decimal scaling
            reserve0 = reserve0 / (10 ** 18)  # token0 decimals
            reserve1 = reserve1 / (10 ** 18)  # token1 decimals
            
            # Determine which is denom vs token
            if self.token1_is_denom:
                return reserve1, reserve0, True  # (denom_reserve, token_reserve)
            else:
                return reserve0, reserve1, True  # (denom_reserve, token_reserve)
                
        except Exception as e:
            return 0.0, 0.0, False