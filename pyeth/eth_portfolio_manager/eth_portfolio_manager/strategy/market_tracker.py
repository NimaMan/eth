"""
Market Tracker Strategy

Objective:
---------
Track market-wide token performance by buying all tokens with trading enabled
and holding them indefinitely for analysis purposes.

Algorithm:
---------
1. For each new token in INIT state:
   - Check if trading is enabled
   - If enabled, generate BUY signal
   - Otherwise, do nothing

2. For BUY_SUBMITTED state:
   - Confirm the buy transaction
   - Record entry metrics

3. For BUY_CONFIRMED state:
   - Hold position indefinitely
   - No sell signals generated
   - Continue tracking performance metrics

4. For other states:
   - No action taken

This strategy serves as a market-wide performance tracker rather than
an active trading strategy. It allows analysis of how tokens perform
over time without exit conditions.

Position State Transitions:
-------------------------
INIT -> BUY_SUBMITTED -> BUY_CONFIRMED (end)

Configuration:
------------
- position_size_eth: Fixed position size for each token purchase
"""

from typing import Optional
from dataclasses import dataclass

from eth_token.erc20_token.erc20_token import ERC20Token, TokenLifecycleState

from eth_portfolio_manager.core.data_models import TokenPositionState
from eth_portfolio_manager.core.token_position import TokenPosition
from eth_portfolio_manager.strategy.base_strategy import BaseStrategy
from eth_portfolio_manager.core.data_models import TradeSignal, TradingDecision


@dataclass
class StrategyConfig:
    position_size_eth: float = 0.01    # Size of each position in ETH


class MarketTracker(BaseStrategy):
    def __init__(self, config: Optional[StrategyConfig] = None):
        self.config = config or StrategyConfig()

    @property
    def strategy_name(self) -> str:
        return "MarketTracker"

    @property
    def strategy_parameters(self) -> dict:
        return {
            "strategy_name": self.strategy_name,
            "position_size_eth": self.config.position_size_eth,
        }
    
    def analyze_token(self, token: ERC20Token, position: TokenPosition) -> Optional[TradeSignal]:
        """
        Analyze token and generate trading signals based on current position state
        
        State Flow:
        INIT -> BUY_SUBMITTED -> BUY_CONFIRMED (end)
        """
        
        # Handle each state explicitly
        if position.latest_snapshot.position_state == TokenPositionState.INIT:
            return self.handle_init_state(token, position)
        
        elif position.latest_snapshot.position_state == TokenPositionState.BUY_SUBMITTED:
            return self.handle_buy_submitted_state(token, position)
        
        # No signals for other states - we just hold positions indefinitely
        return None

    def handle_init_state(self, token: ERC20Token, position: TokenPosition) -> Optional[TradeSignal]:
        """Handle INIT state: Submit buy if trading enabled"""
        if token.token_life_cycle_status == TokenLifecycleState.TRADING_ENABLED:
            return TradeSignal(
                token_address=token.contract_address,
                decision=TradingDecision.SUBMIT_BUY,
                quantity=self.config.position_size_eth,
                strategy_name=self.strategy_name,
            )
        return None

    def handle_buy_submitted_state(self, token: ERC20Token, position: TokenPosition) -> Optional[TradeSignal]:
        """Handle BUY_SUBMITTED state: Confirm buy on next update"""
        return TradeSignal(
            token_address=token.contract_address,
            decision=TradingDecision.CONFIRM_BUY,
            quantity=self.config.position_size_eth,
            strategy_name=self.strategy_name,
        )

    def handle_buy_confirmed_state(self, token: ERC20Token, position: TokenPosition) -> Optional[TradeSignal]:
        """Handle BUY_CONFIRMED state: No action, just hold indefinitely"""
        # No sell signals - we're just tracking market performance
        return None

    def handle_sell_submitted_state(self, token: ERC20Token, position: TokenPosition) -> Optional[TradeSignal]:
        """Handle SELL_SUBMITTED state: Not used in this strategy"""
        # This state should never be reached in this strategy
        return None 
