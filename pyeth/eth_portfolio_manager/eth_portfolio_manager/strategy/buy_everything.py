"""
Simple Buy Strategy

Objective:
---------
1. Buy all newly enabled tokens that meet basic safety criteria
2. Use LiveTokenData for comprehensive token analysis
3. Hold positions indefinitely (no sell signals)
"""

from typing import Dict, List, Optional
from dataclasses import dataclass

from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token
from eth_token_monitor.live_erc20_token.data.live_token_data import LiveTokenData, TokenStatusEnum
from eth_portfolio_manager.strategy.base import BaseStrategy
from eth_portfolio_manager.core.data_models import TradeSignal, TradingDecision
from eth_portfolio_manager.utils.logger import get_logger


@dataclass
class StrategyConfig:
    position_size_eth: float = 0.01    # Size of each position in ETH
    

class JustBuyEverythingStrategy(BaseStrategy):
    def __init__(self, config: Optional[StrategyConfig] = None):
        self.logger = get_logger(name="portfolio_manager", log_folder="portfolio_manager")
        self.config = config or StrategyConfig()
        
    def analyze_token(self, token: LiveERC20Token, position_state) -> Optional[TradeSignal]:
        """
        Analyze token and generate trading signals
        
        Args:
            token: Token data to analyze
            position_state: Current state of our position (if any)
            
        Returns:
            TradeSignal if we should take action, None otherwise
        """
        # Skip if we already have a position
        if position_state.has_active_position:
            return None

        return self.evaluate_buy_action(token)
        
    def evaluate_buy_action(self, token: LiveERC20Token) -> Optional[TradeSignal]:
        """Evaluate if we should buy this token"""
        
        # Check token status
        if token.token_status != TokenStatusEnum.TRADING_ENABLED:
            return None
                    
        # Generate buy signal
        return TradeSignal(
            token_address=token.contract_address,
            decision=TradingDecision.SUBMIT_BUY,
            quantity=self.config.position_size_eth,
            price=token.sync_info.current_price_ratio,
            block_number=token.token_data.latest_block_number,
        )

    def evaluate_sell_action(self, token: LiveERC20Token) -> Optional[TradeSignal]:
        """This strategy doesn't generate sell signals"""
        return None
