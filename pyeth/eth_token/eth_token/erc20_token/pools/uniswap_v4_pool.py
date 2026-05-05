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

from typing import Dict, Optional, Tuple
from dataclasses import dataclass

from eth_token.erc20_token.pools.base_pool import BasePool, logger
from eth_token.erc20_token.pools.addresses import require_checksum_address
from eth_token.erc20_token.pools.numeric import parse_raw_float, parse_raw_int
from eth_token.erc20_token.pools.pool_chain_data_fetcher import PoolChainDataFetcher
from eth_token.erc20_token.token_chain_data_fetcher import TokenChainDataFetcher
from pyreth import PoolBuySellParameters
from eth_data.chain_utils.common_addresses import ZERO_ADDRESS, canonicalize_dex_pool_type


UNISWAP_V4_PROTOCOL = canonicalize_dex_pool_type('UNISWAP-V4')


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
    
    def __init__(
        self,
        pool_id: str,
        pool_key: PoolKey,
        token_address: str,
        denom_address: str,
        *,
        token_decimals: Optional[int] = None,
        denom_decimals: Optional[int] = None,
        token1_is_denom: bool = True,
        pool_chain_fetcher: Optional['PoolChainDataFetcher'] = None,
        token_chain_fetcher: Optional['TokenChainDataFetcher'] = None,
        history_limit: int = 100,
    ):
        super().__init__(
            pool_address=self.POOL_MANAGER,
            token_address=token_address,
            denom_address=denom_address,
            token_decimals=token_decimals,
            denom_decimals=denom_decimals,
            token1_is_denom=token1_is_denom,
            pool_chain_fetcher=pool_chain_fetcher,
            token_chain_fetcher=token_chain_fetcher,
            history_limit=history_limit,
        )
        
        self.pool_id = pool_id
        self.pool_key = pool_key
        
        # Display address for compatibility (PoolManager#poolId format)
        self.display_address = f"{self.POOL_MANAGER}-{pool_id}"
        
        self.sqrt_price_x96: int = 0
        self.current_tick: int = 0
        self.current_liquidity: int = 0
        
        self.ticks: Dict[int, V4Tick] = {}
    
    def get_protocol(self) -> str:
        return UNISWAP_V4_PROTOCOL

    def get_denom_name(self) -> str:
        return super().get_denom_name()
    
    def update_from_transaction(self, transaction: Dict):
        # V4 uses ModifyLiquidity for both mints and burns
        if transaction.get('uniswap_v4_modifies'):
            for modify in transaction['uniswap_v4_modifies']:
                if modify.get('pool_id') == self.pool_id:
                    self._process_modify_liquidity(modify, transaction)

        if transaction.get('uniswap_v4_swaps'):
            for swap in transaction['uniswap_v4_swaps']:
                if swap.get('pool_id') == self.pool_id:
                    self._process_swap(swap, transaction)
        
        # Check if trading is enabled after processing all events
        self.check_and_update_trading_status(transaction)
    
    def _process_swap(self, swap: dict, transaction: Dict):
        """Process a V4 swap event."""
        # Mark token as buyable from first swap event
        self.mark_can_buy_from_event(transaction, event_type='swap')
        
        self.sqrt_price_x96 = parse_raw_int(swap.get('sqrt_price_x96', 0))
        new_tick = parse_raw_int(swap.get('tick', 0))
        
        if self.current_tick != new_tick:
            self.current_liquidity = self._get_liquidity_for_tick(new_tick)
            self.current_tick = new_tick

        self._update_virtual_reserves()
        self._update_prices()

        amount0 = parse_raw_float(swap.get('amount0', 0))
        amount1 = parse_raw_float(swap.get('amount1', 0))
        token_amount, denom_amount = self._map_token_and_denom(amount0, amount1)
        self.state.token_volume_in += max(0.0, token_amount)
        self.state.token_volume_out += max(0.0, -token_amount)
        self.state.denom_volume_in += max(0.0, denom_amount)
        self.state.denom_volume_out += max(0.0, -denom_amount)
        self.state.total_swaps += 1
        
        self._append_event(self.swap_events, dict(swap))

    def _process_modify_liquidity(self, modify: dict, transaction: Dict):
        # NOTE: Modifying liquidity does NOT mean trading is enabled
        # We only mark trading enabled on swaps

        liquidity_delta = parse_raw_int(modify.get('liquidity_delta', 0))
        tick_lower = parse_raw_int(modify.get('tick_lower', 0))
        tick_upper = parse_raw_int(modify.get('tick_upper', 0))
        self.register_token_control_addresses(
            [modify.get('owner')]
            or [modify.get('sender')]
            or [modify.get('recipient')]
            or [modify.get('account')]
        )

        # Positive delta is a mint, negative is a burn
        if liquidity_delta > 0:
            self.state.total_mints += 1
            self._append_event(self.mint_events, dict(modify))
        else:
            self.state.total_burns += 1
            self._append_event(self.burn_events, dict(modify))

        self._update_tick(tick_lower, liquidity_delta)
        self._update_tick(tick_upper, -liquidity_delta)

        if tick_lower <= self.current_tick < tick_upper:
            self.current_liquidity += liquidity_delta
            self.current_liquidity = max(0, self.current_liquidity)

        self._update_virtual_reserves()

    def evaluate_trading_status(self, transaction: Dict) -> None:
        config = PoolBuySellParameters.with_denom_amount(
            float(self.test_buy_amount_eth),
            int(self.get_token_decimals()),
            int(self.get_denom_decimals()),
        )
        denom_address = self.denom_address or self.pool_buy_sell_config.denom_address
        config.denom_address = denom_address
        config.block_number = int(transaction['block_number']) - 1
        prior_transactions = self.latest_block_control_address_txs_list
        result = self.pool_buy_sell_simulator.check_uniswap_v4_pool(
            self.token_address,
            self.POOL_MANAGER,
            self.pool_id,
            config,
            prior_transactions or None,
        )

        if result.can_buy and not self.can_buy:
            self.state.can_buy = self.can_buy = True
            self.can_buy_block = transaction['block_number']
            self.can_buy_tx = transaction['hash']
            self.can_buy_timestamp = transaction.get('block_timestamp', 0)

        self.state.can_sell = self.can_sell = bool(result.can_sell)
        self.buy_tax = result.buy_tax_percentage
        self.sell_tax = result.sell_tax_percentage
        self.tax_check_block = transaction['block_number']
        self.tax_check_tx = transaction['hash']

        logger.info(
            f"TradingStatus "
            f"block={transaction['block_number']} "
            f"token={self.token_address} "
            f"pool_id={self.pool_id} "
            f"from={transaction['from_address']} "
            f"tx={transaction['hash']} "
            f"UniswapV4 Pool "
            f"can_buy={result.can_buy} "
            f"can_sell={result.can_sell} "
            f"buy_tax={result.buy_tax_percentage} "
            f"sell_tax={result.sell_tax_percentage} "
            f"approve={result.can_approve} "
            f"error={result.error_message}"
        )

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
            self.state.token_reserve = 0.0
            self.state.denom_reserve = 0.0
            return

        sqrt_price = self.sqrt_price_x96 / (2**96)
        
        reserve0_raw = self.current_liquidity / sqrt_price
        reserve1_raw = self.current_liquidity * sqrt_price
        
        token0_decimals = self._get_token0_decimals()
        token1_decimals = self._get_token1_decimals()

        reserve0 = reserve0_raw / (10**token0_decimals)
        reserve1 = reserve1_raw / (10**token1_decimals)
        token_reserve, denom_reserve = self._map_token_and_denom(reserve0, reserve1)
        self.state.token_reserve = token_reserve
        self.state.denom_reserve = denom_reserve
        
    def _update_prices(self):
        token_reserve = self.get_token_reserve()
        denom_reserve = self.get_denom_reserve()
        if token_reserve > 0 and denom_reserve > 0:
            self.state.price_denom_per_token = denom_reserve / token_reserve
            self.state.price_token_per_denom = token_reserve / denom_reserve
            self._append_event(self.price_history, (self.state.last_update_block, self.get_price()))

    def _get_token0_decimals(self) -> int:
        currency0 = require_checksum_address(self.pool_key.currency0)
        token_addr = require_checksum_address(self.token_address)
        denom_addr = require_checksum_address(self.denom_address)

        if currency0 == require_checksum_address(ZERO_ADDRESS):
            return 18
        if currency0 == token_addr:
            return self.get_token_decimals()
        if currency0 == denom_addr:
            return self.get_denom_decimals()
        return int(self.token_chain_fetcher.get_token_decimals(currency0))

    def _get_token1_decimals(self) -> int:
        currency1 = require_checksum_address(self.pool_key.currency1)
        token_addr = require_checksum_address(self.token_address)
        denom_addr = require_checksum_address(self.denom_address)

        if currency1 == require_checksum_address(ZERO_ADDRESS):
            return 18
        if currency1 == token_addr:
            return self.get_token_decimals()
        if currency1 == denom_addr:
            return self.get_denom_decimals()
        return int(self.token_chain_fetcher.get_token_decimals(currency1))

    
    # ChainQuery-backed fetchers should be used externally for live reserve data.
    
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
