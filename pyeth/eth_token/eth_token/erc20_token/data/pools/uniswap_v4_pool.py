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

from typing import Dict, Optional, Tuple, List, TYPE_CHECKING
from dataclasses import dataclass
from .base_pool import BasePool

if TYPE_CHECKING:
    from .pool_chain_data_fetcher import PoolChainDataFetcher
    from ..token_chain_data_fetcher import TokenChainDataFetcher

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

    # Discovered mainnet V4 pools for common pairs (via Reth DB logs)
    # Note: V4 uses PoolId (bytes32) rather than pool addresses. These entries
    # help manual wiring when constructing known pools for tokens.
    KNOWN_POOLS: Dict[str, List[Dict[str, str | int]]] = {
        # USDC / WETH
        "USDC_WETH": [
            {
                "pool_id": "0x6d4bc5556c4b1b0d13d58f710e6de12b1d7a0711ef2b95dbf8507e96932162fa",
                "currency0": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
                "currency1": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                "fee": 1000,
                "tick_spacing": 1,
                "hooks": "0x36FABF0DaCD49E94dDb3A21999F199068a9Fe8a8",
            },
            {
                "pool_id": "0xf54122f945300a5230ce2bf95ceb7a70248368fe59125ca43dcf00052054a236",
                "currency0": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
                "currency1": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                "fee": 100,
                "tick_spacing": 1,
                "hooks": "0xf3621C65B9597819a854743ECe1A0f2d62c5A8A8",
            },
        ],
        # USDT / WETH (no Initialize events found in recent history)
        "USDT_WETH": [],
    }
    
    def __init__(
        self,
        pool_id: str,
        pool_key: PoolKey,
        token_address: str,
        denom_address: str,
        token1_is_denom: bool = True,
        pool_chain_fetcher: Optional['PoolChainDataFetcher'] = None,
        token_chain_fetcher: Optional['TokenChainDataFetcher'] = None,
    ):
        super().__init__(
            self.POOL_MANAGER,
            token_address,
            denom_address,
            token1_is_denom,
            pool_chain_fetcher=pool_chain_fetcher,
            token_chain_fetcher=token_chain_fetcher,
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
        return "Uniswap-V4"
    
    def process_transaction(self, transaction: Dict):
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

    def _process_modify_liquidity(self, modify: dict, transaction: Dict):
        # NOTE: Modifying liquidity does NOT mean trading is enabled
        # We only mark trading enabled on swaps

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
        addr = None
        try:
            if self.token1_is_denom:  # Our token is token0 in the pair
                addr = self.token_address if self.token_address == pool_key_token0 else self.denom_address
            else:  # Our token is token1 in the pair
                addr = self.denom_address if self.denom_address == pool_key_token0 else self.token_address
            decimals = self.token_chain_fetcher.get_token_decimals(addr)
        except Exception:
            decimals = None
        return int(decimals) if decimals is not None else 18

    def _get_token1_decimals(self) -> int:
        pool_key_token1 = self.pool_key.currency1
        addr = None
        try:
            if self.token1_is_denom:  # Our token is token0 in the pair
                addr = self.denom_address if self.denom_address == pool_key_token1 else self.token_address
            else:  # Our token is token1 in the pair
                addr = self.token_address if self.token_address == pool_key_token1 else self.denom_address
            decimals = self.token_chain_fetcher.get_token_decimals(addr)
        except Exception:
            decimals = None
        return int(decimals) if decimals is not None else 18
    
    def get_reserves_from_blockchain(self, block_identifier='latest') -> Tuple[float, float, bool]:
        """
        Fetch V4 pool virtual reserves using PyReth ChainQuery (PoolManager + PoolId).
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

            info = self.pool_chain_fetcher.get_v4_liquidity(self.POOL_MANAGER, self.pool_id, block)
            if not info:
                return 0.0, 0.0, False

            reserve0 = info.get('reserve0_scaled') or info.get('reserve0')
            reserve1 = info.get('reserve1_scaled') or info.get('reserve1')
            if reserve0 is None or reserve1 is None:
                L = float(int(info.get('liquidity') or info.get('v3_liquidity') or 0))
                tick = int(info.get('tick', 0))
                sqrt_price = math.pow(1.0001, tick / 2)
                if sqrt_price == 0 or L == 0:
                    return 0.0, 0.0, True
                reserve0_raw = L / sqrt_price
                reserve1_raw = L * sqrt_price
                token0_decimals = int(info.get('token0_decimals') or self.token_chain_fetcher.get_token_decimals(self.pool_key.currency0) or 18)
                token1_decimals = int(info.get('token1_decimals') or self.token_chain_fetcher.get_token_decimals(self.pool_key.currency1) or 18)
                reserve0 = reserve0_raw / (10 ** token0_decimals)
                reserve1 = reserve1_raw / (10 ** token1_decimals)

            # Map based on token1_is_denom and PoolKey currency mapping
            if self.token1_is_denom:
                denom_reserve = reserve1
                token_reserve = reserve0
            else:
                denom_reserve = reserve0
                token_reserve = reserve1

            return float(denom_reserve), float(token_reserve), True
        except Exception:
            return 0.0, 0.0, False
      
    # Removed Web3-based fallbacks; ChainQuery provides the required data
    
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
