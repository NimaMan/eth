"""
Live Token Position Manager

Objective:
---------
1. Track token positions and trading states
2. Apply investment strategies to evaluate positions
3. Create and update positions based on token updates
"""

from dataclasses import dataclass
from typing import Dict, List, Optional
from datetime import datetime
from enum import Enum

from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token
from eth_token_monitor.live_erc20_token.data.live_token_data import TokenStatusEnum
from eth_portfolio_manager.core.data_models import TradingDecision, TokenPositionData, TokenPositionState
from eth_portfolio_manager.strategy.buy_everything import JustBuyEverythingStrategy


STRATEGY_NAME = "LiveTokenPositionManager"
STRATEGY = JustBuyEverythingStrategy


class LiveTokenPositionManager:
    def __init__(self):
        self.investment_strategy = STRATEGY()
        
    async def process_token_updates(self, updated_token: LiveERC20Token, current_position: TokenPositionData) -> TokenPositionData:
        """Process token updates and manage positions
            - Update position state form the token data
            - Apply investment strategy to generate trade signals
            - Update position data based on trade signals
        """
        
        # Update position state
        updated_position = self._update_position_state(current_position, updated_token)
        
        # Apply investment strategy
        signal = self.investment_strategy.analyze_token(
            token=updated_token,
            position_state=updated_position
        )
        
        # Update position based on signals
        if signal:
            updated_position = self._update_position_from_signal(signal, updated_position, updated_token)
            
        return updated_position
    
    def _update_position_state(self, position: TokenPositionData, token: LiveERC20Token) -> TokenPositionData:
        """Update position state based on current position and the token data
        - Update position state based on token status
            - TokenStatusEnum.INACTIVE_SCAM: Update position state to scammed
        - If not, update the position based on the latest token data
        """
        # Check token status
        if token.token_status == TokenStatusEnum.INACTIVE_SCAM:
            position = self._update_scammed_position(position, token)
            return position
        return self._update_position_from_token_data(position, token)
    
    def _update_scammed_position(self, position: TokenPositionData, token: LiveERC20Token) -> TokenPositionData:
        """Update position state to scammed"""
        position.position_state = TokenPositionState.SCAMMED
        position.Xprice = 0
        position.current_value = 0
        position.realized_profit  = - position.purchase_value
        position.unrealized_profit = 0
        position.token_age_blocks =  token.token_age_blocks
        position.token_age_hours = token.token_age_hours
        position.last_updated_block = token.token_data.latest_block_number
        position.last_updated_time = token.token_data.latest_block_timestamp
        position.has_active_position = False
        
        return position
    
    def _update_position_from_token_data(self, position: TokenPositionData, token: LiveERC20Token):
        """Update position state based on current position and the token data"""
        position.current_Xprice = token.sync_info.current_price_ratio
        position.Xprice = position.current_Xprice / position.entry_Xprice if position.current_Xprice else 0
        position.current_value = 0.01 * position.Xprice
        position.realized_profit = 0
        position.unrealized_profit = position.purchase_value * position.Xprice
        position.token_age_blocks = token.token_trading_age_blocks
        position.token_age_hours = token.token_trading_age_hours
        position.last_updated_block = token.token_data.latest_block_number
        position.last_updated_time = token.token_data.latest_block_timestamp
        
        return position
    
    def _update_position_from_signal(self, signal: TradingDecision, position: TokenPositionData, token: LiveERC20Token):
        """Update position state based on current position and the signal
            - if the signal is set to submit buy, then the position state is set to BUY_SUBMITTED
            - if the signal is set to submit sell, then the position state is set to SELL_SUBMITTED
            - if the signal is set to buy confirmed, then the position state is set to BUY_CONFIRMED
            - if the signal is set to sell confirmed, then the position state is set to SELL_CONFIRMED
        """
        if signal.decision == TradingDecision.SUBMIT_BUY:
            if not position.has_active_position:
                # New position
                position.has_active_position = True
                position.position_state = TokenPositionState.BUY_SUBMITTED
                position.entry_block = token.token_data.latest_block_number
                position.entry_Xprice = token.sync_info.current_price_ratio
                position.current_Xprice = token.sync_info.current_price_ratio
                position.Xprice = position.current_Xprice / position.entry_Xprice
                position.purchase_value = 0.01 * position.Xprice
                position.current_value = 0.01 * position.Xprice
                position.realized_profit = 0
                position.unrealized_profit = 0.01 * position.Xprice
            elif signal.decision == TradingDecision.CONFIRM_BUY:
                # Add to position
                position.has_active_position = True
                position.position_state = TokenPositionState.BUY_CONFIRMED
                position.entry_Xprice = token.sync_info.current_price_ratio
                position.current_Xprice = token.sync_info.current_price_ratio
                position.Xprice = position.current_Xprice / position.entry_Xprice
                position.purchase_value = 0.01 * position.Xprice
                position.current_value = 0.01 * position.Xprice
                position.realized_profit = 0
                position.unrealized_profit = 0.01 * position.Xprice
                position.last_updated_block = token.token_data.latest_block_number
                position.last_updated_time = token.token_data.latest_block_timestamp

            elif signal.decision == TradingDecision.SUBMIT_SELL:
                position.position_state = TokenPositionState.SELL_SUBMITTED
                position.has_active_position = True
                position.last_updated_block = token.token_data.latest_block_number
                position.last_updated_time = token.token_data.latest_block_timestamp

            elif signal.decision == TradingDecision.CONFIRM_SELL:
                position.position_state = TokenPositionState.SELL_CONFIRMED
                position.has_active_position = False
                position.last_updated_block = token.token_data.latest_block_number
                position.last_updated_time = token.token_data.latest_block_timestamp
                
        return position
    
    