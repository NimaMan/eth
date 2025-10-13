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
import pyreth

from .base_pool import BasePool, logger
from eth_data.chain_utils.common_addresses import canonicalize_dex_pool_type

UNISWAP_V3_PROTOCOL = canonicalize_dex_pool_type('UNISWAP-V3')

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
        *,
        token_decimals: Optional[int] = None,
        denom_decimals: Optional[int] = None,
        token1_is_denom: bool = True,
        fee_tier: int = 3000,
        pool_chain_fetcher: Optional['PoolChainDataFetcher'] = None,
        token_chain_fetcher: Optional['TokenChainDataFetcher'] = None,
        history_limit: int = 100,
    ):
        super().__init__(
            pool_address=pool_address,
            token_address=token_address,
            denom_address=denom_address,
            token_decimals=token_decimals,
            denom_decimals=denom_decimals,
            token1_is_denom=token1_is_denom,
            pool_chain_fetcher=pool_chain_fetcher,
            token_chain_fetcher=token_chain_fetcher,
            history_limit=history_limit,
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
        return UNISWAP_V3_PROTOCOL
    
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
        self._append_event(self.swap_events, {
            'block': transaction['block_number'],
            'tx_hash': transaction['hash'],
            'amount0': amount0,
            'amount1': amount1,
            'sqrt_price_x96': self.sqrt_price_x96,
            'liquidity': self.current_liquidity,
            'tick': self.current_tick,
            'timestamp': transaction['block_timestamp']
        })
    
    def evaluate_trading_status(self, transaction: Dict) -> None:
        self.pool_buy_sell_config.test_amount_eth = float(self.test_buy_amount_eth)
        self.pool_buy_sell_config.token_decimals = int(self.get_token_decimals())
        self.pool_buy_sell_config.block_number = int(transaction['block_number'])
        if transaction.get('block_header'):
            self.pool_buy_sell_config.set_block_header(transaction['block_header'])

        result = self.pool_buy_sell_simulator.check_uniswap_v3_pool(
                self.token_address,
                self.pool_address,
                int(self.fee_tier),
                self.pool_buy_sell_config,
            )
        
        if result.can_buy and not self.can_buy:
            self.can_buy = True
            self.can_buy_block = transaction['block_number']
            self.can_buy_tx = transaction['hash']
            self.can_buy_timestamp = transaction.get('block_timestamp', 0)

        # Update sell status and tax rates
        self.can_sell = bool(result.can_sell)
        self.buy_tax = result.buy_tax_percentage
        self.sell_tax = result.sell_tax_percentage
        self.tax_check_block = transaction['block_number']
        self.tax_check_tx = transaction['hash']
    
        logger.info(
            f"UniswapV3 Pool:"
            f"tx={transaction.get('hash')} "
            f"token={self.token_address} "
            f"pool={self.pool_address} "
            f"block={transaction['block_number']} "
            f"can_buy={result.can_buy} "
            f"can_sell={result.can_sell} "
            f"buy_tax={result.buy_tax_percentage} "
            f"sell_tax={result.sell_tax_percentage} "
            f"approve={result.can_approve} "
            f"error={result.error_message}"
        )
    
    def _process_mint(self, mint: dict, transaction: Dict):
        """Process a V3 mint (add liquidity) event."""
        # NOTE: Adding liquidity does NOT mean trading is enabled
        # Trading might still be disabled - we only mark trading enabled on swaps
        
        liquidity_delta = int(mint.get('amount', 0))
        tick_lower = int(mint.get('tick_lower', 0))
        tick_upper = int(mint.get('tick_upper', 0))
        self._token_control_addresses.add(mint.get('owner', mint.get('to_address')))

        self._update_tick(tick_lower, liquidity_delta)
        self._update_tick(tick_upper, -liquidity_delta)

        # If current price is within this new position's range, update active liquidity
        if tick_lower <= self.current_tick < tick_upper:
            self.current_liquidity += liquidity_delta
            
        self._update_virtual_reserves()
        self.state.total_mints += 1
        
        # Store mint event
        self._append_event(self.mint_events, {
            'block': transaction['block_number'],
            'tx_hash': transaction['hash'],
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
        self._append_event(self.burn_events, {
            'block': transaction['block_number'],
            'tx_hash': transaction['hash'],
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
            self._append_event(self.price_history, (self.state.last_update_block, self.get_price()))

    def _get_token0_decimals(self) -> int:
        """Get token0 decimals based on configuration."""
        return self.get_token_decimals() if self.token1_is_denom else self.get_denom_decimals()

    def _get_token1_decimals(self) -> int:
        """Get token1 decimals based on configuration."""
        return self.get_denom_decimals() if self.token1_is_denom else self.get_token_decimals()
    
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
