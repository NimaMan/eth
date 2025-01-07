"""
Simple Buy and Sell Strategy

Objective:
---------
1. Buy all newly enabled tokens immediately
2. Fixed position size for all trades
3. Sell when price ratio (Xprice) exceeds threshold

Algorithm:
---------
1. Buy Conditions:
   - Token is trading enabled
   - No active position exists
   - Use fixed position size

2. Sell Conditions:
   - Have active position
   - Current price / Entry price >= threshold
   - Sell entire position
"""

from typing import Optional
from dataclasses import dataclass

from eth_portfolio_manager.core.data_models import TokenPositionData
from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token
from eth_token_monitor.live_erc20_token.data.live_token_data import LiveTokenData, TokenStatusEnum
from eth_portfolio_manager.strategy.base import BaseStrategy
from eth_portfolio_manager.core.data_models import TradeSignal, TradingDecision
from eth_portfolio_manager.utils.logger import get_logger


@dataclass
class StrategyConfig:
    position_size_eth: float = 0.01    # Size of each position in ETH
    profit_target_x: float = 10.0      # Sell when price increases by this multiple
    

class JustBuyEverythingStrategy(BaseStrategy):
    def __init__(self, config: Optional[StrategyConfig] = None):
        self.logger = get_logger(name="portfolio_manager", log_folder="portfolio_manager")
        self.config = config or StrategyConfig()
        
    def analyze_token(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
        """
        Analyze token and generate trading signals
        
        Args:
            token: Token data to analyze
            position_state: Current state of our position (if any)
        """
        # Check for sell signal if we have a position
        if position_state.has_active_position:
            return self.evaluate_sell_action(token, position_state)

        # Check for buy signal if we don't have a position
        return self.evaluate_buy_action(token)
        
    def evaluate_buy_action(self, token: LiveERC20Token) -> Optional[TradeSignal]:
        """Generate buy signal if token is trading enabled"""
        
        # Only check if trading is enabled
        if token.token_status != TokenStatusEnum.TRADING_ENABLED:
            return None
                    
        # Generate buy signal with current price
        return TradeSignal(
            token_address=token.contract_address,
            decision=TradingDecision.SUBMIT_BUY,
            quantity=self.config.position_size_eth,
            strategy_name=self.__class__.__name__,
        )

    def evaluate_sell_action(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
        """Generate sell signal if price target is reached"""
        # Calculate current price ratio
        price_ratio = position_state.Xprice
        
        # Sell if we hit our target
        if price_ratio >= self.config.profit_target_x:
            return TradeSignal(
                token_address=token.contract_address,
                decision=TradingDecision.SUBMIT_SELL,
                quantity=0,
                strategy_name=self.__class__.__name__,
            )
            
        return None
