"""
Backtest Token Position Manager

Objective:
---------
1. Track token positions and trading states
2. Apply investment strategies to evaluate positions
3. Create and update positions based on token updates

Position State Flow:
------------------
1. Token Creation & Initial State
   - New token detected -> Create TokenPositionData with INIT state
   - Tracked in BacktestPositionManager
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

from typing import Dict, List 

from eth_portfolio_manager.strategy.base_strategy import BaseStrategy
from eth_token.erc20_token.erc20_token import ERC20Token
from eth_portfolio_manager.core.data_models import TradeSignal, TradingDecision, TokenPositionState
from eth_portfolio_manager.core.token_position import TokenPosition


class BacktestStrategyEngine:
    def __init__(self, investment_strategy: BaseStrategy):
        self.investment_strategy = investment_strategy
    
    @property
    def strategy_parameters(self):
        return self.investment_strategy.strategy_parameters
    
    @property
    def strategy_name(self):
        return self.investment_strategy.strategy_name
    
    def process_updated_token(self, live_token: ERC20Token, token_position: TokenPosition) -> TokenPosition:
        """Process token updates and manage positions
            - Apply investment strategy to generate trade signals
            - Update position data based on trade signals
        """
        # Apply investment strategy
        signal = self.investment_strategy.analyze_token(token=live_token, position=token_position)
        # Update position based on signals
        if signal:
            token_position = self._update_position_from_signal(signal=signal, token_position=token_position, live_token=live_token)
            
        return token_position
    
    def _update_position_from_signal(self, signal: TradeSignal, token_position: TokenPosition, live_token: ERC20Token):
        """Update position state based on current position and the signal
            - if the signal is set to submit buy, then the position state is set to BUY_SUBMITTED
            - if the signal is set to submit sell, then the position state is set to SELL_SUBMITTED
            - if the signal is set to buy confirmed, then the position state is set to BUY_CONFIRMED
            - if the signal is set to sell confirmed, then the position state is set to SELL_CONFIRMED
        """
        if signal.decision == TradingDecision.SUBMIT_BUY:
            token_position = self.update_submit_buy(token_position, live_token)
        
        elif signal.decision == TradingDecision.CONFIRM_BUY:
            token_position = self.update_confirm_buy(token_position, live_token)

        elif signal.decision == TradingDecision.SUBMIT_SELL:
            token_position = self.update_submit_sell(token_position, live_token)
        
        elif signal.decision == TradingDecision.CONFIRM_SELL:
            token_position = self.update_confirm_sell(token_position, live_token)
        
        return token_position
    
    def update_submit_buy(self, token_position: TokenPosition, live_token: ERC20Token) -> TokenPosition:
        """
        Update position state when submitting a buy order using token information directly.
        
        State Transition: INIT -> BUY_SUBMITTED
        
        This function:
          - Records entry static data from live_token.
          - Updates the existing latest snapshot in place.
          - Avoids creating an extra snapshot object.
        """
        if token_position.latest_snapshot.position_state == TokenPositionState.INIT:
            current_price_ratio = 0
            if token_position.static_data.pool_address:
                current_price_ratio = live_token.token_data.latest_pools_price_ratio.get(token_position.static_data.pool_address, 0)
            # Record entry static data
            token_position.static_data.entry_block = live_token.token_data.latest_block_number
            token_position.static_data.entry_price_ratio = current_price_ratio
            token_position.static_data.purchase_value = self.investment_strategy.config.position_size_eth

            # Update the latest snapshot in place
            token_position.latest_snapshot.position_state = TokenPositionState.BUY_SUBMITTED
            token_position.latest_snapshot.has_active_position = True
            token_position.latest_snapshot.current_price_ratio = current_price_ratio
            token_position.latest_snapshot.roi = 0.0  # ROI equals 0 at entry (i.e. 1 - 1 = 0)
            token_position.latest_snapshot.current_value = token_position.static_data.purchase_value
            token_position.latest_snapshot.realized_profit = 0
            token_position.latest_snapshot.unrealized_profit = 0

        return token_position

    def update_confirm_buy(self, token_position: TokenPosition, live_token: ERC20Token) -> TokenPosition:
        """
        Update position state when buy order is confirmed using token data directly.
        
        State Transition: BUY_SUBMITTED -> BUY_CONFIRMED
        
        This function:
          - Updates the latest snapshot in place with blockchain-confirmed data.
          - Recalculates ROI and current value based on the confirmed price.
        """
        if token_position.latest_snapshot.position_state == TokenPositionState.BUY_SUBMITTED:
            current_price_ratio = 0
            if token_position.static_data.pool_address:
                current_price_ratio = live_token.token_data.latest_pools_price_ratio.get(token_position.static_data.pool_address, 0)
            token_position.latest_snapshot.position_state = TokenPositionState.BUY_CONFIRMED

            token_position.static_data.entry_block = live_token.token_data.latest_block_number
            token_position.static_data.entry_price_ratio = current_price_ratio
            token_position.static_data.purchase_value = self.investment_strategy.config.position_size_eth

            # Update the latest snapshot in place
            token_position.latest_snapshot.has_active_position = True
            token_position.latest_snapshot.current_price_ratio = current_price_ratio
            token_position.latest_snapshot.roi = 0.0  # ROI equals 0 at entry (i.e. 1 - 1 = 0)
            token_position.latest_snapshot.current_value = token_position.static_data.purchase_value
            token_position.latest_snapshot.realized_profit = 0
            token_position.latest_snapshot.unrealized_profit = 0
            
        return token_position

    def update_submit_sell(self, token_position: TokenPosition, live_token: ERC20Token) -> TokenPosition:
        """
        Update position state when submitting a sell order using token information directly.
        
        State Transition: BUY_CONFIRMED -> SELL_SUBMITTED
        
        This function:
          - Records exit static data from live_token.
          - Updates the latest snapshot in place to reflect the sell submission.
        """
        if token_position.latest_snapshot.position_state == TokenPositionState.BUY_CONFIRMED:
            current_price_ratio = 0
            if token_position.static_data.pool_address:
                current_price_ratio = live_token.token_data.latest_pools_price_ratio.get(token_position.static_data.pool_address, 0)
            # Record exit static data directly from the token data
            token_position.static_data.exit_block = live_token.token_data.latest_block_number
            token_position.static_data.exit_price_ratio = current_price_ratio
            token_position.static_data.exit_timestamp = live_token.token_data.latest_block_timestamp
            
            # Update the latest snapshot in place for sell submission
            token_position.latest_snapshot.position_state = TokenPositionState.SELL_SUBMITTED
            token_position.latest_snapshot.has_active_position = False
            token_position.latest_snapshot.current_price_ratio = current_price_ratio
            token_position.latest_snapshot.realized_profit = token_position.latest_snapshot.current_value - token_position.static_data.purchase_value
            token_position.latest_snapshot.unrealized_profit = 0
            age_blocks, age_hours = token_position.get_trading_ages(live_token)
            token_position.latest_snapshot.token_age_blocks = age_blocks
            token_position.latest_snapshot.token_age_hours = age_hours

        return token_position

    def update_confirm_sell(self, token_position: TokenPosition, live_token: ERC20Token) -> TokenPosition:
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
        if token_position.latest_snapshot.position_state == TokenPositionState.SELL_SUBMITTED:
            token_position.latest_snapshot.position_state = TokenPositionState.SELL_CONFIRMED
            token_position.latest_snapshot.has_active_position = False

            # Record exit static data
            token_position.static_data.exit_block = live_token.token_data.latest_block_number
                        
        return token_position
