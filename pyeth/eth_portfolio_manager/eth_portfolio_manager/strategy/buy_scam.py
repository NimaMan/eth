"""
Buy Scam Strategy

Position State Transitions:
-------------------------
1. Position Creation (INIT)
   - New token detected
   - Position created in INIT state
   - No active position
   - Waiting for trading to be enabled

2. Buy Submission (INIT -> BUY_SUBMITTED)
   Trigger: Token is flagged as Particular SCAM
   Actions that we know:
   - Generate SUBMIT_BUY signal
   - Record entry price attempt
   - Mark position as active
   - Set initial position size (0.01 ETH)

3. Buy Confirmation (BUY_SUBMITTED -> BUY_CONFIRMED)
   Trigger: Next update after BUY_SUBMITTED
   Actions:
   - Generate CONFIRM_BUY signal
   - Finalize entry price
   - Begin tracking position value
   - Calculate unrealized profit/loss

4. Sell Submission (BUY_CONFIRMED -> SELL_SUBMITTED)
   Trigger: Price ratio (Xprice) >= profit_target_x
   Actions:
   - Generate SUBMIT_SELL signal
   - Record exit price attempt
   - Prepare for position closure
   - Continue tracking unrealized P/L

5. Sell Confirmation (SELL_SUBMITTED -> SELL_CONFIRMED)
   Trigger: Next update after SELL_SUBMITTED
   Actions:
   - Generate CONFIRM_SELL signal
   - Finalize exit price
   - Calculate realized profit
   - Mark position as inactive

Special Cases:
------------
- SCAM Detection: Any state can transition to SCAMMED
- Only evaluate sell signals in BUY_CONFIRMED state
- Must confirm buy before allowing sell signals
- Position remains active until sell is confirmed
"""

from typing import Optional
from dataclasses import dataclass

from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token
from eth_token_monitor.live_erc20_token.data.live_token_data import TokenStatusEnum

from eth_portfolio_manager.core.data_models import TokenPositionData, TokenPositionState
from eth_portfolio_manager.strategy.base import BaseStrategy
from eth_portfolio_manager.core.data_models import TradeSignal, TradingDecision


@dataclass
class StrategyConfig:
    position_size_eth: float = 0.01    # Size of each position in ETH
    profit_target_x: float = 5.0      # Sell when price increases by this multiple
    

class BuyScamStrategy(BaseStrategy):
    def __init__(self, config: Optional[StrategyConfig] = None):
        self.config = config or StrategyConfig()
        
    def analyze_token(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
        """
        Analyze token and generate trading signals based on current position state
        
        State Flow:
        INIT -> BUY_SUBMITTED -> BUY_CONFIRMED -> SELL_SUBMITTED -> SELL_CONFIRMED (end)
        """
        
        # Handle each state explicitly
        if position_state.position_state == TokenPositionState.INIT:
            return self.handle_init_state(token, position_state)
        
        elif position_state.position_state == TokenPositionState.BUY_SUBMITTED:
            return self.handle_buy_submitted_state(token, position_state)
        
        elif position_state.position_state == TokenPositionState.BUY_CONFIRMED:
            return self.handle_buy_confirmed_state(token, position_state)
        
        elif position_state.position_state == TokenPositionState.SELL_SUBMITTED:
            return self.handle_sell_submitted_state(token, position_state)
        
        return None

    def handle_init_state(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
        """Handle INIT state: Submit buy if trading enabled"""
        if token.latest_token_assessment.get('is_scam'):
            return TradeSignal(
                token_address=token.contract_address,
                decision=TradingDecision.SUBMIT_BUY,
                quantity=self.config.position_size_eth,
                strategy_name=self.__class__.__name__,
            )
        return None

    def handle_buy_submitted_state(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
        """Handle BUY_SUBMITTED state: Confirm buy on next update"""
        return TradeSignal(
            token_address=token.contract_address,
            decision=TradingDecision.CONFIRM_BUY,
            quantity=self.config.position_size_eth,
            strategy_name=self.__class__.__name__,
        )

    def handle_buy_confirmed_state(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
        """Handle BUY_CONFIRMED state: Submit sell if price target reached"""
        if position_state.Xprice >= self.config.profit_target_x:
            return TradeSignal(
                token_address=token.contract_address,
                decision=TradingDecision.SUBMIT_SELL,
                quantity=position_state.quantity,
                strategy_name=self.__class__.__name__,
            )
        return None

    def handle_sell_submitted_state(self, token: LiveERC20Token, position_state: TokenPositionData) -> Optional[TradeSignal]:
        """Handle SELL_SUBMITTED state: Confirm sell on next update"""
        return TradeSignal(
            token_address=token.contract_address,
            decision=TradingDecision.CONFIRM_SELL,
            quantity=position_state.quantity,
            strategy_name=self.__class__.__name__,
        )
