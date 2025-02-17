"""
Base Strategy Interface

Objective:
---------
Define the base interface for all trading strategies that handle position lifecycle events.

Strategy Event Flow:
-----------------
1. Token Analysis (analyze_token)
   - Entry point for all token updates
   - Receives current token data and position state
   - Routes to appropriate state handler
   - Returns trading signals based on state

2. State Handlers:
   a. INIT State:
      - New token detected
      - Evaluate trading conditions
      - Generate buy signal if conditions met
   
   b. BUY_SUBMITTED State:
      - Buy transaction pending
      - Confirm entry on next update
      - Track entry price and position size
   
   c. BUY_CONFIRMED State:
      - Active position
      - Monitor price movements
      - Generate sell signals based on strategy
   
   d. SELL_SUBMITTED State:
      - Sell transaction pending
      - Confirm exit on next update
      - Finalize position metrics

Required Methods:
--------------
1. analyze_token(token, position_state) -> Optional[TradeSignal]
   - Main entry point for token updates
   - Routes to appropriate state handler
   - Returns trading signals

2. handle_init_state(token, position_state) -> Optional[TradeSignal]
   - Evaluates initial buy conditions
   - Generates buy signals

3. handle_buy_submitted_state(token, position_state) -> Optional[TradeSignal]
   - Confirms buy transactions
   - Updates position entry data

4. handle_buy_confirmed_state(token, position_state) -> Optional[TradeSignal]
   - Monitors active positions
   - Generates sell signals

5. handle_sell_submitted_state(token, position_state) -> Optional[TradeSignal]
   - Confirms sell transactions
   - Finalizes position exit
"""

from abc import ABC, abstractmethod
from typing import Optional

from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token
from eth_portfolio_manager.core.data_models import TokenPositionData, TradeSignal


class BaseStrategy(ABC):
   @abstractmethod
   def analyze_token(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
      """Main entry point for token analysis and signal generation"""
      pass

   @abstractmethod
   def handle_init_state(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
      """Handle INIT state and evaluate buy conditions"""
      pass

   @abstractmethod
   def handle_buy_submitted_state(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
      """Handle BUY_SUBMITTED state and confirm entries"""
      pass

   @abstractmethod
   def handle_buy_confirmed_state(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
      """Handle BUY_CONFIRMED state and evaluate sell conditions"""
      pass

   @abstractmethod
   def handle_sell_submitted_state(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
      """Handle SELL_SUBMITTED state and confirm exits"""
      pass

   @property
   def strategy_name(self) -> str:
      """Return the name of the strategy"""
      return self.__class__.__name__

   @property
   def strategy_parameters(self) -> dict:
      """Return the parameters of the strategy"""
      return {}
   
   def get_parameters(self) -> dict:
      """Return the parameters of the strategy"""
      return self.strategy_parameters