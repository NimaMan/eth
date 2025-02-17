"""
Live Token Position Manager

Objective:
---------
1. Track token positions and trading states
2. Apply investment strategies to evaluate positions
3. Create and update positions based on token updates

Position State Flow:
------------------
1. Token Creation & Initial State
   - New token detected -> Create TokenPositionData with INIT state
   - Tracked in PortfolioPositionManager
   - No active position yet

2. Position Updates from Token Data
   - Regular updates from LiveTokenManager via PortfolioManager
   - Updates token metrics (age, block numbers, timestamps)
   - Updates price data if available
   - No state change unless trading signal received

3. Trading Signal State Transitions:
   INIT -> BUY_SUBMITTED:
   - Strategy generates SUBMIT_BUY signal
   - Records entry attempt details
   - Sets position as active
   
   BUY_SUBMITTED -> BUY_CONFIRMED:
   - Our buy transaction is detected
   - Updates final entry price and position size
   - Calculates initial position metrics
   
   BUY_CONFIRMED -> SELL_SUBMITTED:
   - Strategy generates SUBMIT_SELL signal
   - Records exit attempt details
   - Prepares for position closure
   
   SELL_SUBMITTED -> SELL_CONFIRMED:
   - Our sell transaction is detected
   - Finalizes exit price and realized profit
   - Closes the position

4. Special State Transitions:
   ANY_STATE -> SCAMMED:
   - Token detected as scam
   - Position marked as inactive
   - Records realized losses

Event Handling:
-------------
1. Token Updates (No Signal):
   - Source: LiveTokenManager -> PortfolioManager
   - Handler: process_token_updates()
   - Action: Updates position metrics without state change
   - Files: live_token_position_manager.py, portfolio_position_manager.py

2. Trading Signals:
   - Source: Strategy (e.g., buy_everything.py)
   - Handler: _update_position_from_signal()
   - Action: Triggers state transition based on signal
   - Files: live_token_position_manager.py, strategy implementations

3. Scam Detection:
   - Source: LiveTokenData status updates
   - Handler: _update_position_state()
   - Action: Moves position to SCAMMED state
   - Files: live_token_data.py, live_token_position_manager.py

4. Position Metrics:
   - Source: Token price/sync updates
   - Handler: _update_position_from_token_data()
   - Action: Updates position values and profits
   - Files: live_token.py, live_token_position_manager.py

Data Flow:
---------
1. Token Creation:
   LiveTokenManager -> PortfolioManager -> PortfolioPositionManager
   -> Creates new position in INIT state

2. Regular Updates:
   LiveTokenManager -> PortfolioManager -> PortfolioPositionManager
   -> LiveTokenPositionManager -> Updates metrics

3. Trading Signals:
   Strategy -> LiveTokenPositionManager -> Updates state
   -> PortfolioPositionManager -> Updates storage

4. Position Storage:
   PortfolioPositionManager -> PortfolioStateServer
   -> Persists position updates to Redis

Key Metrics Tracked:
------------------
1. Position State (TokenPositionState enum)
2. Entry/Current Prices (Xprice metrics)
3. Position Value (purchase_value, current_value)
4. Profits (realized_profit, unrealized_profit)
5. Token Age (blocks and hours)
6. Update Timestamps (blocks and time)

Implementation Notes:
------------------
1. All position updates are atomic
2. State transitions are strictly controlled
3. Metrics are updated with every token update
4. Historical data is maintained for analysis
5. Error states are properly handled
"""

from dataclasses import dataclass
from typing import Dict, List, Optional
from datetime import datetime
from enum import Enum

from eth_portfolio_manager.strategy.base import BaseStrategy
from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token
from eth_token_monitor.live_erc20_token.data.live_token_data import TokenStatusEnum
from eth_portfolio_manager.core.data_models import TradingDecision, TokenPositionData, TokenPositionState



class LiveTokenPositionManager:
    def __init__(self, investment_strategy: BaseStrategy):
        self.investment_strategy = investment_strategy()
        
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
        
        State Updates by Position State:
        ------------------------------
        1. ANY_STATE -> SCAMMED:
           - Token detected as scam
           - Zero out position value
           - Record realized losses
           
        2. INIT:
           - Update basic token metrics
           - No price/value calculations yet
           
        3. BUY_SUBMITTED:
           - Update token metrics
           - Track price changes before confirmation
           - No value/profit calculations yet
           
        4. BUY_CONFIRMED:
           - Update token metrics
           - Calculate current position value
           - Track unrealized profits
           
        5. SELL_SUBMITTED:
           - Update token metrics
           - Continue tracking position value
           - Prepare for exit price calculation
           
        6. SELL_CONFIRMED:
           - Update final metrics
           - Calculate realized profits
           - Position marked as inactive
        """
        # First check for scam status
        if token.token_status == TokenStatusEnum.INACTIVE_SCAM:
            return self._update_scammed_position(position, token)
        
        # Then update based on current position state
        return self._update_position_from_token_data(position, token)
    
    def _update_scammed_position(self, position: TokenPositionData, token: LiveERC20Token) -> TokenPositionData:
        """Update position state to scammed
        
        Actions:
        1. Mark position as SCAMMED state
        2. Zero out current value and price ratios
        3. Move all value to realized losses
        4. Update token metrics
        5. Mark position as inactive
        """
        position.position_state = TokenPositionState.SCAMMED
        position.Xprice = 0
        position.current_value = 0
        position.realized_profit = -position.purchase_value  # Full loss
        position.unrealized_profit = 0
        position.token_age_blocks = token.token_age_blocks
        position.token_age_hours = token.token_age_hours
        position.block_number = token.token_data.latest_block_number
        position.last_updated_time = token.token_data.latest_block_timestamp
        position.has_active_position = False
        
        return position
    
    def _update_position_from_token_data(self, position: TokenPositionData, token: LiveERC20Token):
        """Update position state based on current position and the token data
        
        Updates by Position State:
        ------------------------
        1. INIT/BUY_SUBMITTED:
           - Update token metrics only
           - Track price for entry
           
        2. BUY_CONFIRMED:
           - Update token metrics
           - Calculate current position value
           - Track unrealized profit/loss
           
        3. SELL_SUBMITTED:
           - Similar to BUY_CONFIRMED
           - Continue tracking until confirmation
           
        4. SELL_CONFIRMED:
           - Update final metrics only
           - No value/profit updates
        """
        # Always update token metrics
        position.token_age_blocks = token.token_trading_age_blocks
        position.token_age_hours = token.token_trading_age_hours
        position.block_number = token.token_data.latest_block_number
        position.last_updated_time = token.token_data.latest_block_timestamp
        position.current_Xprice = token.sync_info.current_price_ratio
            
        # Update price and value metrics based on position state
        if position.has_active_position:
            # Active position updates
            position.Xprice = position.current_Xprice / position.entry_Xprice if position.entry_Xprice else 0
            position.current_value = position.purchase_value * position.Xprice
            position.unrealized_profit = position.current_value - position.purchase_value
                
        return position
    
    def _update_position_from_signal(self, signal: TradingDecision, position: TokenPositionData, token: LiveERC20Token):
        """Update position state based on current position and the signal
            - if the signal is set to submit buy, then the position state is set to BUY_SUBMITTED
            - if the signal is set to submit sell, then the position state is set to SELL_SUBMITTED
            - if the signal is set to buy confirmed, then the position state is set to BUY_CONFIRMED
            - if the signal is set to sell confirmed, then the position state is set to SELL_CONFIRMED
        """
        if signal.decision == TradingDecision.SUBMIT_BUY:
            position = self.update_submit_buy(position, token)
        
        elif signal.decision == TradingDecision.CONFIRM_BUY:
            position = self.update_confirm_buy(position, token)

        elif signal.decision == TradingDecision.SUBMIT_SELL:
            position = self.update_submit_sell(position, token)
        
        elif signal.decision == TradingDecision.CONFIRM_SELL:
            position = self.update_confirm_sell(position, token)
        
        return position
    
    def update_submit_buy(self, position: TokenPositionData, token: LiveERC20Token) -> TokenPositionData:
        """Update position state when submitting a buy order
        
        State Transition: INIT -> BUY_SUBMITTED
        
        This function:
        1. Updates position state to BUY_SUBMITTED
        2. Records entry price and block
        3. Calculates initial position metrics
        4. Sets position as active

        In production:
        - Would be called before submitting transaction to chain
        - Next state should be BUY_CONFIRMED once our transaction is mined
        """
        if position.position_state == TokenPositionState.INIT:
            position.position_state = TokenPositionState.BUY_SUBMITTED
            position.has_active_position = True
            
            # Record entry data
            position.entry_block = token.token_data.latest_block_number
            position.entry_Xprice = token.sync_info.current_price_ratio
            
            # Update current metrics
            position.current_Xprice = token.sync_info.current_price_ratio
            position.Xprice = 1.0  # At entry, current price = entry price
            
            # Set initial position size (0.01 ETH worth)
            position.purchase_value = 0.01
            position.current_value = 0.01
            position.realized_profit = 0
            position.unrealized_profit = 0
            
            # Update timestamp
            position.block_number = token.token_data.latest_block_number
            position.last_updated_time = token.token_data.latest_block_timestamp
            
        return position

    def update_confirm_buy(self, position: TokenPositionData, token: LiveERC20Token) -> TokenPositionData:
        """Update position state when buy order is confirmed
        
        State Transition: BUY_SUBMITTED -> BUY_CONFIRMED
        
        This function:
        1. Updates position state to BUY_CONFIRMED
        2. Verifies and updates final entry price
        3. Updates position metrics with confirmed values
        
        In production:
        - Would be called after our buy transaction is mined
        - Confirms actual entry price and position size
        """
        if position.position_state == TokenPositionState.BUY_SUBMITTED:
            position.position_state = TokenPositionState.BUY_CONFIRMED
            
            # Update current metrics
            position.current_Xprice = token.sync_info.current_price_ratio
            position.Xprice = position.current_Xprice / position.entry_Xprice
            
            # Update value and profit calculations
            position.current_value = position.purchase_value * position.Xprice
            position.unrealized_profit = position.current_value - position.purchase_value
            
            # Update timestamp
            position.block_number = token.token_data.latest_block_number
            position.last_updated_time = token.token_data.latest_block_timestamp
            
        return position

    def update_submit_sell(self, position: TokenPositionData, token: LiveERC20Token) -> TokenPositionData:
        """Update position state when submitting a sell order
        
        State Transition: BUY_CONFIRMED -> SELL_SUBMITTED
        
        This function:
        1. Updates position state to SELL_SUBMITTED
        2. Records exit price attempt
        3. Prepares for position closure
        
        In production:
        - Would be called before submitting sell transaction
        - Next state should be SELL_CONFIRMED once our transaction is mined
        """
        if position.position_state == TokenPositionState.BUY_CONFIRMED:
            position.position_state = TokenPositionState.SELL_SUBMITTED
            
            # Update current metrics
            position.current_Xprice = token.sync_info.current_price_ratio
            position.Xprice = position.current_Xprice / position.entry_Xprice
            
            # Calculate current position value
            position.current_value = position.purchase_value * position.Xprice
            position.unrealized_profit = position.current_value - position.purchase_value
            
            # Update timestamp
            position.block_number = token.token_data.latest_block_number
            position.last_updated_time = token.token_data.latest_block_timestamp
            
        return position

    def update_confirm_sell(self, position: TokenPositionData, token: LiveERC20Token) -> TokenPositionData:
        """Update position state when sell order is confirmed
        
        State Transition: SELL_SUBMITTED -> SELL_CONFIRMED
        
        This function:
        1. Updates position state to SELL_CONFIRMED
        2. Finalizes exit price and realized profit
        3. Closes the position
        
        In production:
        - Would be called after our sell transaction is mined
        - Finalizes actual exit price and realized profit
        """
        if position.position_state == TokenPositionState.SELL_SUBMITTED:
            position.position_state = TokenPositionState.SELL_CONFIRMED
            position.has_active_position = False
            
            # Calculate final position value and profit
            position.current_Xprice = token.sync_info.current_price_ratio
            position.Xprice = position.current_Xprice / position.entry_Xprice
            position.current_value = position.purchase_value * position.Xprice
            position.realized_profit = position.current_value - position.purchase_value
            position.unrealized_profit = 0
            
            # Update timestamp
            position.block_number = token.token_data.latest_block_number
            position.last_updated_time = token.token_data.latest_block_timestamp
            
        return position