"""
Uniswap V3 Pool Implementation

Key Features:
- Concentrated liquidity with tick-based positions
- Dynamic fee tiers (0.01%, 0.05%, 0.3%, 1%)
- Position tracking with tick ranges
- Virtual reserves calculated from current price and liquidity
"""

from typing import Optional, Dict, List, Tuple, TYPE_CHECKING
from dataclasses import dataclass, field
import math

from .base_pool import BasePool

if TYPE_CHECKING:
    from .pool_chain_data_fetcher import PoolChainDataFetcher
    from ..token_chain_data_fetcher import TokenChainDataFetcher


@dataclass
class Position:
    """Represents a V3 liquidity position."""
    owner: str
    liquidity: int
    tick_lower: int
    tick_upper: int

@dataclass
class Tick:
    """Represents a tick with its total liquidity."""
    liquidity_gross: int = 0
    liquidity_net: int = 0


class UniswapV3Pool(BasePool):
    """
    Uniswap V3 pool that correctly processes concentrated liquidity events.
    """
    
    def __init__(
        self,
        pool_address: str,
        token_address: str,
        denom_address: str,
        token1_is_denom: bool = True,
        fee_tier: int = 3000,
        pool_chain_fetcher: Optional['PoolChainDataFetcher'] = None,
        token_chain_fetcher: Optional['TokenChainDataFetcher'] = None,
    ):
        super().__init__(
            pool_address,
            token_address,
            denom_address,
            token1_is_denom,
            pool_chain_fetcher=pool_chain_fetcher,
            token_chain_fetcher=token_chain_fetcher,
        )
        
        self.fee_tier = fee_tier
        self.sqrt_price_x96: int = 0
        self.current_tick: int = 0
        self.current_liquidity: int = 0
        self.tick_spacing = self._get_tick_spacing(fee_tier)
        
        # Core data structures for concentrated liquidity
        self.ticks: Dict[int, Tick] = {}
        self.positions: Dict[str, Position] = {} # owner -> Position
        
        # Event storage
        self.swap_events = []
        self.mint_events = []
        self.burn_events = []
        
    def get_protocol(self) -> str:
        return "Uniswap-V3"
    
    def _get_tick_spacing(self, fee: int) -> int:
        """Get tick spacing for fee tier."""
        return {
            100: 1,     # 0.01%
            500: 10,    # 0.05%
            3000: 60,   # 0.30%
            10000: 200  # 1.00%
        }.get(fee, 60)
    
    def process_transaction(self, transaction: Dict):
        """
        Process V3 events from a transaction.
        
        V3 events:
        - uniswap_v3_swaps: Swaps with tick/liquidity data
        - uniswap_v3_mints: Position creations (if present)
        - uniswap_v3_burns: Position removals (if present)
        """
        # Correctly ordered processing: Mints/Burns first, then Swaps
        # Process v3 mints
        if transaction.get('uniswap_v3_mints'):
            for mint in transaction['uniswap_v3_mints']:
                if mint.get('pool_address', '') == self.pool_address:
                    self._process_mint(mint, transaction)
                
        # Process v3 burns
        if transaction.get('uniswap_v3_burns'):
            for burn in transaction['uniswap_v3_burns']:
                if burn.get('pool_address', '') == self.pool_address:
                    self._process_burn(burn, transaction)

        # Process v3 swaps
        if transaction.get('uniswap_v3_swaps'):
            for swap in transaction['uniswap_v3_swaps']:
                if swap.get('pool_address', '') == self.pool_address:
                    self._process_swap(swap, transaction)
        
        # Check if trading is enabled after processing all events
        self.check_and_update_trading_status(transaction)
    
    def _process_swap(self, swap: dict, transaction: Dict):
        """Process a V3 swap event."""
        # Mark token as buyable from first swap event
        self.mark_can_buy_from_event(transaction, event_type='swap')
        
        self.sqrt_price_x96 = int(swap.get('sqrt_price_x96', 0))
        new_tick = int(swap.get('tick', 0))
        
        # Update active liquidity if tick is crossed
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
        
        # Store swap event
        self.swap_events.append({
            'block': transaction['block_number'],
            'txn_hash': transaction['hash'],
            'amount0': amount0,
            'amount1': amount1,
            'sqrt_price_x96': self.sqrt_price_x96,
            'liquidity': self.current_liquidity,
            'tick': self.current_tick,
            'timestamp': transaction['block_timestamp']
        })
    
    def _process_mint(self, mint: dict, transaction: Dict):
        """Process a V3 mint (add liquidity) event."""
        # NOTE: Adding liquidity does NOT mean trading is enabled
        # Trading might still be disabled - we only mark trading enabled on swaps
        
        liquidity_delta = int(mint.get('amount', 0))
        tick_lower = int(mint.get('tick_lower', 0))
        tick_upper = int(mint.get('tick_upper', 0))

        self._update_tick(tick_lower, liquidity_delta)
        self._update_tick(tick_upper, -liquidity_delta)

        # If current price is within this new position's range, update active liquidity
        if tick_lower <= self.current_tick < tick_upper:
            self.current_liquidity += liquidity_delta
            
        self._update_virtual_reserves()
        self.state.total_mints += 1
        
        # Store mint event
        self.mint_events.append({
            'block': transaction['block_number'],
            'txn_hash': transaction['hash'],
            'owner': mint.get('owner', mint.get('to_address')),
            'amount': liquidity_delta,
            'tick_lower': tick_lower,
            'tick_upper': tick_upper,
            'timestamp': transaction['block_timestamp']
        })
    
    def _process_burn(self, burn: dict, transaction: Dict):
        """Process a V3 burn (remove liquidity) event."""
        # NOTE: Removing liquidity does NOT indicate trading status
        # We only mark trading enabled on swaps
        
        liquidity_delta = int(burn.get('amount', 0))
        tick_lower = int(burn.get('tick_lower', 0))
        tick_upper = int(burn.get('tick_upper', 0))

        self._update_tick(tick_lower, -liquidity_delta)
        self._update_tick(tick_upper, liquidity_delta)

        # If current price is within this removed position's range, update active liquidity
        if tick_lower <= self.current_tick < tick_upper:
            self.current_liquidity = max(0, self.current_liquidity - liquidity_delta)

        self._update_virtual_reserves()
        self.state.total_burns += 1
        
        # Store burn event
        self.burn_events.append({
            'block': transaction['block_number'],
            'txn_hash': transaction['hash'],
            'owner': burn.get('owner', burn.get('from_address')),
            'amount': liquidity_delta,
            'tick_lower': tick_lower,
            'tick_upper': tick_upper,
            'timestamp': transaction['block_timestamp']
        })
    
    def _update_tick(self, tick_index: int, liquidity_delta: int):
        if tick_index not in self.ticks:
            self.ticks[tick_index] = Tick()
        
        tick = self.ticks[tick_index]
        tick.liquidity_gross += abs(liquidity_delta)
        tick.liquidity_net += liquidity_delta

    def _get_liquidity_for_tick(self, tick_index: int) -> int:
        active_liquidity = self.current_liquidity
        
        # Simplified: assumes ticks are processed in order. For a full historical rebuild,
        # a more robust method of iterating through all ticks up to the current one is needed.
        sorted_ticks = sorted(self.ticks.keys())
        
        start_index = 0
        try:
            start_index = sorted_ticks.index(self.current_tick)
        except ValueError:
            # If current tick not in our list, we have to search from beginning
            pass
            
        for i in range(start_index, len(sorted_ticks)):
            t = sorted_ticks[i]
            if t > tick_index:
                break
            active_liquidity += self.ticks[t].liquidity_net
            
        return active_liquidity

    def _update_virtual_reserves(self):
        """
        Calculate virtual reserves for V2 compatibility.
        
        For V3 pools, virtual reserves are calculated from current price and liquidity.
        This provides compatibility with V2-style reserve queries.
        """
        if self.sqrt_price_x96 == 0 or self.current_liquidity == 0:
            self.state.reserve0 = 0
            self.state.reserve1 = 0
            return
            
        sqrt_price = self.sqrt_price_x96 / (2**96)
        
        # Formulas from Uniswap V3 whitepaper to calculate virtual reserves
        # reserve0 = liquidity / sqrt_price
        # reserve1 = liquidity * sqrt_price
        reserve0_raw = self.current_liquidity / sqrt_price
        reserve1_raw = self.current_liquidity * sqrt_price
        
        token0_decimals = self._get_token0_decimals()
        token1_decimals = self._get_token1_decimals()

        # Update state with properly scaled reserves
        self.state.reserve0 = reserve0_raw / (10**token0_decimals)
        self.state.reserve1 = reserve1_raw / (10**token1_decimals)
        
    def _update_prices(self):
        if self.state.reserve0 > 0 and self.state.reserve1 > 0:
            self.state.price0 = self.state.reserve1 / self.state.reserve0
            self.state.price1 = self.state.reserve0 / self.state.reserve1
            self.price_history.append((self.state.last_update_block, self.get_price()))

    def _get_token0_decimals(self) -> int:
        """Get token0 decimals based on configuration."""
        decimals = None
        try:
            if self.token1_is_denom:
                decimals = self.token_chain_fetcher.get_token_decimals(self.token_address)
            else:
                decimals = self.token_chain_fetcher.get_token_decimals(self.denom_address)
        except Exception:
            decimals = None
        return int(decimals) if decimals is not None else 18

    def _get_token1_decimals(self) -> int:
        """Get token1 decimals based on configuration."""
        decimals = None
        try:
            if self.token1_is_denom:
                decimals = self.token_chain_fetcher.get_token_decimals(self.denom_address)
            else:
                decimals = self.token_chain_fetcher.get_token_decimals(self.token_address)
        except Exception:
            decimals = None
        return int(decimals) if decimals is not None else 18
    
    def get_current_tick(self) -> int:
        """Get current tick."""
        return self.current_tick
    
    def get_current_liquidity(self) -> int:
        """Get current active liquidity."""
        return self.current_liquidity
    
    def get_sqrt_price_x96(self) -> int:
        """Get current sqrt price in X96 format."""
        return self.sqrt_price_x96
    
    def calculate_price_from_tick(self, tick: int) -> float:
        """Calculate price from tick."""
        # price = 1.0001^tick
        return 1.0001 ** tick
    
    def calculate_tick_from_price(self, price: float) -> int:
        """Calculate tick from price."""
        if price <= 0:
            return 0
        # tick = log(price) / log(1.0001)
        return int(math.log(price) / math.log(1.0001))
    
    def get_price_range(self) -> Tuple[float, float]:
        """Get price range for current tick spacing."""
        min_tick = (self.current_tick // self.tick_spacing) * self.tick_spacing
        max_tick = min_tick + self.tick_spacing
        
        min_price = self.calculate_price_from_tick(min_tick)
        max_price = self.calculate_price_from_tick(max_tick)
        
        return min_price, max_price
    
    def is_healthy(self) -> bool:
        """Check if pool is healthy."""
        # V3 pools are healthy if they have liquidity and recent swaps
        if self.current_liquidity == 0:
            return False
        
        if self.state.total_swaps == 0:
            return False
            
        return True
    
    def get_reserves_from_blockchain(self, block_identifier='latest') -> Tuple[float, float, bool]:
        """
        Fetch virtual reserves for V3 using PyReth ChainQuery (tick + liquidity).
        
        Args:
            block_identifier: Block number or 'latest'
            
        Returns:
            Tuple of (denom_reserve, token_reserve, success)
        """
        try:
            import math
            # Resolve block number; None implies latest
            block = None
            if isinstance(block_identifier, int):
                block = int(block_identifier)
            elif isinstance(block_identifier, str) and block_identifier != 'latest':
                try:
                    block = int(block_identifier)
                except Exception:
                    block = None

            info = self.pool_chain_fetcher.get_v3_liquidity(self.pool_address, int(self.fee_tier), block)
            if not info or info.get('liquidity') is None or info.get('tick') is None or not info.get('token0') or not info.get('token1'):
                return 0.0, 0.0, False

            token0_addr = info['token0']
            token1_addr = info['token1']
            reserve0 = info.get('reserve0_scaled') or info.get('reserve0')
            reserve1 = info.get('reserve1_scaled') or info.get('reserve1')
            if reserve0 is None or reserve1 is None:
                L = float(int(info['liquidity']))
                tick = int(info['tick'])
                sqrt_price = math.pow(1.0001, tick / 2)
                if sqrt_price == 0 or L == 0:
                    return 0.0, 0.0, True
                reserve0_raw = L / sqrt_price
                reserve1_raw = L * sqrt_price
                token0_decimals = int(info.get('token0_decimals') or self.token_chain_fetcher.get_token_decimals(token0_addr) or 18)
                token1_decimals = int(info.get('token1_decimals') or self.token_chain_fetcher.get_token_decimals(token1_addr) or 18)
                reserve0 = reserve0_raw / (10 ** token0_decimals)
                reserve1 = reserve1_raw / (10 ** token1_decimals)

            # Map to denom/token based on which side denom is on
            if token0_addr.lower() == self.denom_address.lower():
                denom_reserve = reserve0
                token_reserve = reserve1
            elif token1_addr.lower() == self.denom_address.lower():
                denom_reserve = reserve1
                token_reserve = reserve0
            else:
                return 0.0, 0.0, False

            return float(denom_reserve), float(token_reserve), True
        except Exception:
            return 0.0, 0.0, False
