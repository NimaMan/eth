"""
Base Strategy Interface

Objective:
---------
Define the base interface for all trading strategies
"""

from abc import ABC, abstractmethod
from typing import Dict, List, Optional

from eth_token_monitor.live_erc20_token.data.live_token_data import LiveTokenData
from eth_portfolio_manager.core.data_models import TradeSignal


class BaseStrategy(ABC):
    @abstractmethod
    def analyze_token(self, token: LiveTokenData, position_state: Dict[str, any]) -> Optional[List[TradeSignal]]:
        """Analyze token and generate trading signals"""
        pass

    @abstractmethod
    def evaluate_buy_action(self, token: LiveTokenData) -> Optional[TradeSignal]:
        """Evaluate if we should buy this token"""
        pass

    @abstractmethod
    def evaluate_sell_action(self, token: LiveTokenData) -> Optional[TradeSignal]:
        """Evaluate if we should sell this token"""
        pass

    