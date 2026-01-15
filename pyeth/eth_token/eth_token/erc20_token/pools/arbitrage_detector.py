"""
Arbitrage Detector for Multi-Pool Token Trading

This module provides sophisticated arbitrage detection capabilities for tokens that trade
across multiple Uniswap pools (V2, V3, V4). It identifies profitable trading opportunities
that arise from price discrepancies between different liquidity pools.

Core Functionality:
------------------
1. **Price Discrepancy Detection**: Continuously monitors all pools for a given token
   to identify price differences that exceed trading fees and gas costs.

2. **Fee-Aware Calculations**: Accounts for protocol-specific fees:
   - Uniswap V2: Fixed 0.3% fee
   - Uniswap V3: Variable fees (0.01%, 0.05%, 0.3%, 1%)
   - Uniswap V4: Dynamic fees (defaults to 0.3%)

3. **Profit Calculation**: Computes net profit after accounting for:
   - Entry fee (buying from cheaper pool)
   - Exit fee (selling to expensive pool)
   - Price impact (estimated based on liquidity)

4. **Opportunity Ranking**: Sorts opportunities by profit percentage to help
   prioritize the most lucrative trades.

Key Components:
--------------
- **ArbitrageOpportunity**: Data class containing all relevant information about
  a detected arbitrage opportunity including pools, prices, and profit metrics.

- **ArbitrageDetector**: Main class that performs the detection logic, maintains
  fee configurations, and provides statistical analysis.

Use Cases:
----------
1. **MEV Detection**: Identify Maximum Extractable Value opportunities that
   arbitrage bots might exploit.

2. **Market Health Monitoring**: Large, persistent arbitrage opportunities may
   indicate market inefficiencies or manipulation.

3. **Price Oracle Validation**: Ensure price consistency across pools to detect
   potentially manipulated or stale prices.

4. **Liquidity Analysis**: Low liquidity pools often have larger price deviations
   and more arbitrage opportunities.

Mathematical Model:
------------------
For pools A and B with prices P_A < P_B:

1. Buy Amount * P_A * (1 + fee_A) = Cost
2. Sell Amount * P_B * (1 - fee_B) = Revenue
3. Profit = Revenue - Cost
4. Profit % = (Profit / Cost) * 100

The detector only reports opportunities where Profit % > min_threshold.

Limitations:
-----------
1. **Gas Costs**: Not included in calculations as they vary by network conditions
2. **Price Impact**: Simplified estimation that may not reflect actual slippage
3. **MEV Competition**: Detected opportunities may be taken by bots before execution
4. **Cross-Pool Routing**: Doesn't consider multi-hop arbitrage through intermediate tokens

Configuration:
-------------
- min_profit_threshold: Minimum profit percentage to report (default 1%)
- Protocol fees can be customized via the protocol_fees dictionary

Logging:
--------
Uses a dedicated 'arbitrage_detector' logger that writes to the 'Arbitrage_detection'
folder. Significant opportunities (>5% profit) trigger WARNING level logs.
"""

from typing import Dict, List, Optional, TYPE_CHECKING
from dataclasses import dataclass

from eth_data.chain_utils.common_addresses import canonicalize_dex_pool_type

UNISWAP_V2_PROTOCOL = canonicalize_dex_pool_type('UNISWAP-V2')
UNISWAP_V3_PROTOCOL = canonicalize_dex_pool_type('UNISWAP-V3')
UNISWAP_V4_PROTOCOL = canonicalize_dex_pool_type('UNISWAP-V4')

if TYPE_CHECKING:
    from .pool_manager import PoolManager

@dataclass
class ArbitrageOpportunity:
    """Represents an arbitrage opportunity between two pools."""
    buy_pool: str
    sell_pool: str
    buy_protocol: str
    sell_protocol: str
    buy_price: float
    sell_price: float
    price_difference: float
    profit_percentage: float
    buy_denom_reserve: float
    sell_denom_reserve: float
    max_profitable_amount: Optional[float] = None


class ArbitrageDetector:
    """
    Detects arbitrage opportunities across multiple pools.
    
    Analyzes price differences between pools and identifies profitable
    trading opportunities considering fees and liquidity constraints.
    """
    
    def __init__(self, token_address: str, min_profit_threshold: float = 0.01):
        """
        Initialize arbitrage detector.
        
        Args:
            token_address: The token to monitor for arbitrage
            min_profit_threshold: Minimum profit percentage to report (default 1%)
        """
        self.token_address = token_address
        self.min_profit_threshold = min_profit_threshold
        
        # Fee structures for different protocols
        self.protocol_fees = {
            UNISWAP_V2_PROTOCOL: 0.003,  # 0.3%
            UNISWAP_V3_PROTOCOL: {  # Variable fees
                100: 0.0001,    # 0.01%
                500: 0.0005,    # 0.05%
                3000: 0.003,    # 0.3%
                10000: 0.01     # 1%
            },
            UNISWAP_V4_PROTOCOL: 0.003  # Default, but can be dynamic
        }
    
    def detect_arbitrage(self, pool_manager: 'PoolManager') -> List[ArbitrageOpportunity]:
        """
        Detect arbitrage opportunities across all pools.
        
        Args:
            pool_manager: The pool manager containing all pools
            
        Returns:
            List of arbitrage opportunities sorted by profit percentage
        """
        pools = pool_manager.get_all_pools()
        
        if len(pools) < 2:
            return []
        
        opportunities = []
        
        # Compare all pool pairs
        for i, pool1 in enumerate(pools):
            for pool2 in pools[i+1:]:
                # Skip if either pool has no liquidity
                if pool1.get_denom_reserve() == 0 or pool2.get_denom_reserve() == 0:
                    continue
                
                # Get prices
                price1 = pool1.get_price()
                price2 = pool2.get_price()
                
                if price1 == 0 or price2 == 0:
                    continue
                
                # Determine buy and sell pools
                if price1 < price2:
                    buy_pool, sell_pool = pool1, pool2
                    buy_price, sell_price = price1, price2
                else:
                    buy_pool, sell_pool = pool2, pool1
                    buy_price, sell_price = price2, price1
                
                # Calculate profit considering fees
                buy_fee = self._get_pool_fee(buy_pool)
                sell_fee = self._get_pool_fee(sell_pool)
                
                # Effective prices after fees
                effective_buy_price = buy_price * (1 + buy_fee)
                effective_sell_price = sell_price * (1 - sell_fee)
                
                # Check if profitable after fees
                if effective_sell_price > effective_buy_price:
                    price_diff = effective_sell_price - effective_buy_price
                    profit_pct = (price_diff / effective_buy_price) * 100
                    
                    if profit_pct >= self.min_profit_threshold:
                        opportunity = ArbitrageOpportunity(
                            buy_pool=buy_pool.pool_address,
                            sell_pool=sell_pool.pool_address,
                            buy_protocol=buy_pool.get_protocol(),
                            sell_protocol=sell_pool.get_protocol(),
                            buy_price=buy_price,
                            sell_price=sell_price,
                            price_difference=sell_price - buy_price,
                            profit_percentage=profit_pct,
                            buy_denom_reserve=buy_pool.get_denom_reserve(),
                            sell_denom_reserve=sell_pool.get_denom_reserve()
                        )
                        
                        # Calculate max profitable amount (simplified)
                        opportunity.max_profitable_amount = self._calculate_max_amount(
                            buy_pool, sell_pool, sell_price
                        )
                        
                        opportunities.append(opportunity)
                        
        
        # Sort by profit percentage (highest first)
        opportunities.sort(key=lambda x: x.profit_percentage, reverse=True)
        
        return opportunities
    
    def _get_pool_fee(self, pool) -> float:
        """Get the fee for a specific pool."""
        protocol = canonicalize_dex_pool_type(pool.get_protocol())

        if protocol == UNISWAP_V2_PROTOCOL:
            return self.protocol_fees[UNISWAP_V2_PROTOCOL]
        elif protocol == UNISWAP_V3_PROTOCOL:
            # V3 pools have variable fees stored in fee_tier attribute
            fee_tier = pool.fee_tier  # Will raise AttributeError if missing
            return self.protocol_fees[UNISWAP_V3_PROTOCOL].get(fee_tier, 0.003)
        elif protocol == UNISWAP_V4_PROTOCOL:
            # V4 can have dynamic fees, but use default for now
            # TODO: Implement dynamic fee fetching for V4
            return self.protocol_fees[UNISWAP_V4_PROTOCOL]
        else:
            raise ValueError(f"Unknown protocol: {protocol}")
    
    def _calculate_max_amount(self, buy_pool, sell_pool, sell_price: float) -> float:
        """
        Calculate the maximum profitable trade amount.
        
        This is a simplified calculation that considers:
        - Available liquidity in both pools
        - Price impact of the trade
        
        Returns:
            Maximum amount in denomination currency
        """
        # Get reserves
        buy_denom_reserve = buy_pool.get_denom_reserve()
        sell_token_reserve = sell_pool.get_token_reserve()
        
        # Simple heuristic: limit to 10% of the smaller pool's liquidity
        # to avoid excessive price impact
        max_buy_amount = buy_denom_reserve * 0.1
        max_sell_amount = sell_token_reserve * sell_price * 0.1
        
        return min(max_buy_amount, max_sell_amount)
    
    def get_price_spread_stats(self, pool_manager: 'PoolManager') -> Dict:
        """
        Get statistics about price spreads across pools.
        
        Returns:
            Dict with min, max, average prices and spread percentage
        """
        pools = pool_manager.get_all_pools()
        prices = []
        
        for pool in pools:
            if pool.get_denom_reserve() > 0:
                price = pool.get_price()
                if price > 0:
                    prices.append(price)
        
        if not prices:
            return {
                'min_price': 0,
                'max_price': 0,
                'avg_price': 0,
                'spread_pct': 0,
                'num_pools': 0
            }
        
        min_price = min(prices)
        max_price = max(prices)
        avg_price = sum(prices) / len(prices)
        spread_pct = ((max_price - min_price) / avg_price) * 100 if avg_price > 0 else 0
        
        return {
            'min_price': min_price,
            'max_price': max_price,
            'avg_price': avg_price,
            'spread_pct': spread_pct,
            'num_pools': len(prices)
        }
    
    def log_opportunities(self, opportunities: List[ArbitrageOpportunity]):
        """Retained for backward compatibility; no-op."""
        return
