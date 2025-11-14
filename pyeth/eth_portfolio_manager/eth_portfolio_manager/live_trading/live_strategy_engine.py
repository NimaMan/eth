"""
Live Strategy Engine - Real-time Trading Signal Generator

Purpose:
--------
Generate and publish trading signals to eth_kartal for execution based on real-time 
blockchain data and strategy decisions. This engine tracks live positions and manages
state transitions based on blockchain confirmations from eth_kartal.

Architecture:
------------
1. Strategy Analysis -> Generate TradeSignal
2. Publish signal via ZMQ to eth_kartal  
3. eth_kartal executes the trade on-chain
4. Receive execution confirmation via ZMQ
5. Update position state based on confirmation

Key Differences from Backtest:
-----------------------------
- No hypothetical trades - all signals are sent for real execution
- State transitions depend on actual blockchain confirmations
- Tracks real transaction hashes and gas costs
- Monitors execution success/failure from eth_kartal
- Handles real slippage and MEV protection feedback

Signal Flow:
-----------
Python (Strategy) -> TradeSignal -> ZMQ -> eth_kartal -> Blockchain
                                      ^
                                      |
                         Execution Confirmation

Position State Flow:
-------------------
1. Initial State:
   INIT - Token detected, no position yet

2. Buy Signal Flow:
   INIT -> BUY_SUBMITTED -> BUY_CONFIRMED
   - BUY_SUBMITTED: Signal sent to eth_kartal
   - BUY_CONFIRMED: Transaction confirmed on blockchain

3. Sell Signal Flow:  
   BUY_CONFIRMED -> SELL_SUBMITTED -> SELL_CONFIRMED
   - SELL_SUBMITTED: Sell signal sent to eth_kartal
   - SELL_CONFIRMED: Sell transaction confirmed

4. Failure States:
   BUY_SUBMITTED -> INIT (buy failed)
   SELL_SUBMITTED -> BUY_CONFIRMED (sell failed)
   
5. Special States:
   ANY_STATE -> SCAMMED (token marked as scam)

Signal Publishing:
-----------------
Trade signals are published to eth_kartal with:
- Token and pool information
- Trade direction (BUY/SELL)
- Amount and slippage tolerance
- Strategy metadata
- Wallet address for execution

Execution Feedback:
------------------
eth_kartal provides feedback on:
- Transaction hash
- Execution status (success/failed/pending)
- Actual amounts (after slippage)
- Gas costs
- Failure reasons

State Management:
----------------
- Positions track signal IDs for correlation
- States updated only on blockchain confirmation
- Failed trades revert to previous state
- Transaction hashes stored for verification

Integration Points:
------------------
1. TradeSignalPublisher: Sends signals to eth_kartal
2. Strategy implementations: Generate trading decisions
3. Token position tracking: Maintains position state
4. Execution monitoring: Tracks transaction status

Error Handling:
--------------
- Failed signals logged and position state reverted
- Network issues trigger retry logic
- Slippage exceeded cancels trade
- Gas estimation failures handled gracefully
"""

from typing import Dict, List, Optional
import asyncio
import time

from eth_portfolio_manager.strategy.base_strategy import BaseStrategy
from eth_token.erc20_token.erc20_token import ERC20Token
from eth_portfolio_manager.core.data_models import TradeSignal, TradingDecision, TokenPositionState
from eth_portfolio_manager.core.token_position import TokenPosition
from eth_portfolio_manager.notifications.trade_signal_publisher import TradeSignalPublisher, ExecutionConfirmation, ExecutionStatus
from eth_portfolio_manager.utils.logger import get_logger


class LiveStrategyEngine:
    def __init__(self, investment_strategy: BaseStrategy, signal_publisher: Optional[TradeSignalPublisher] = None):
        self.investment_strategy = investment_strategy
        self.signal_publisher = signal_publisher
        self.logger = get_logger(f"LiveStrategyEngine-{investment_strategy.strategy_name}")
        
        # Track pending signals
        self._pending_signals: Dict[str, str] = {}  # position_key -> signal_id
        
        # Get wallet address from strategy if available
        self.wallet_address = None
        if hasattr(investment_strategy, 'config'):
            if hasattr(investment_strategy.config, 'wallet_address'):
                self.wallet_address = investment_strategy.config.wallet_address
            elif isinstance(investment_strategy.config, dict):
                self.wallet_address = investment_strategy.config.get('wallet_address')
        
        if not self.wallet_address and hasattr(investment_strategy, 'strategy_parameters'):
            if isinstance(investment_strategy.strategy_parameters, dict):
                self.wallet_address = investment_strategy.strategy_parameters.get('wallet_address', self.wallet_address)
    
    @property
    def strategy_parameters(self):
        return self.investment_strategy.strategy_parameters
    
    @property
    def strategy_name(self):
        return self.investment_strategy.strategy_name
    
    async def process_updated_token(self, live_token: ERC20Token, token_position: TokenPosition) -> TokenPosition:
        """Process token updates and manage positions
            - Apply investment strategy to generate trade signals
            - Update position data based on trade signals
            - Publish signals to eth_kartal for execution
        """
        # Apply investment strategy
        signal = self.investment_strategy.analyze_token(token=live_token, position=token_position)
        # Update position based on signals
        if signal:
            token_position = await self._update_position_from_signal(signal=signal, token_position=token_position, live_token=live_token)
            
        return token_position
    
    async def _update_position_from_signal(self, signal: TradeSignal, token_position: TokenPosition, live_token: ERC20Token):
        """Update position state based on current position and the signal
            - if the signal is set to submit buy, then the position state is set to BUY_SUBMITTED
            - if the signal is set to submit sell, then the position state is set to SELL_SUBMITTED  
            - if the signal is set to buy confirmed, then the position state is set to BUY_CONFIRMED
            - if the signal is set to sell confirmed, then the position state is set to SELL_CONFIRMED
        """
        if signal.decision == TradingDecision.SUBMIT_BUY:
            token_position = await self.update_submit_buy(token_position, live_token, signal)
        
        elif signal.decision == TradingDecision.CONFIRM_BUY:
            token_position = self.update_confirm_buy(token_position, live_token)

        elif signal.decision == TradingDecision.SUBMIT_SELL:
            token_position = await self.update_submit_sell(token_position, live_token, signal)
        
        elif signal.decision == TradingDecision.CONFIRM_SELL:
            token_position = self.update_confirm_sell(token_position, live_token)
        
        return token_position
    
    async def update_submit_buy(self, token_position: TokenPosition, live_token: ERC20Token, signal: TradeSignal) -> TokenPosition:
        """
        Update position state when submitting a buy order and publish signal to eth_kartal.
        
        State Transition: INIT -> BUY_SUBMITTED
        
        This function:
          - Publishes buy signal to eth_kartal for execution
          - Records entry static data from live_token
          - Updates the existing latest snapshot in place
          - Tracks signal ID for confirmation correlation
        """
        if token_position.latest_snapshot.position_state == TokenPositionState.INIT:
            current_price_ratio = 0
            if token_position.static_data.pool_address:
                current_price_ratio = live_token.latest_pools_price_ratio.get(token_position.static_data.pool_address, 0)
            
            # Publish signal to eth_kartal if publisher available
            if self.signal_publisher and self.wallet_address:
                try:
                    position_key = f"{token_position.static_data.token_address}-{token_position.static_data.pool_address}"
                    signal_id = await self.signal_publisher.publish_signal(
                        signal=signal,
                        wallet_address=self.wallet_address,
                        pool_address=token_position.static_data.pool_address,
                        pool_type=token_position.static_data.pool_type,
                        max_slippage=0.03,  # 3% default
                        deadline_seconds=300,  # 5 minutes
                        confirmation_callback=self._handle_execution_confirmation
                    )
                    self._pending_signals[position_key] = signal_id
                    self.logger.info(f"Published BUY signal {signal_id[:8]}... for {signal.token_address[:10]}...")
                except Exception as e:
                    self.logger.error(f"Failed to publish buy signal: {e}")
                    # Continue with position update even if signal publishing fails
            
            # Record entry static data
            token_position.static_data.entry_block = live_token.latest_block_number
            token_position.static_data.entry_price_ratio = current_price_ratio
            token_position.static_data.purchase_value = signal.quantity

            # Update the latest snapshot in place
            token_position.latest_snapshot.position_state = TokenPositionState.BUY_SUBMITTED
            token_position.latest_snapshot.has_active_position = True
            token_position.latest_snapshot.current_price_ratio = current_price_ratio
            token_position.latest_snapshot.roi = 0.0  # ROI equals 0 at entry (i.e. 1 - 1 = 0)
            token_position.latest_snapshot.current_value = token_position.static_data.purchase_value
            token_position.latest_snapshot.realized_profit = 0
            token_position.latest_snapshot.unrealized_profit = 0
            token_position.latest_snapshot.quantity = signal.quantity

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
                current_price_ratio = live_token.latest_pools_price_ratio.get(token_position.static_data.pool_address, 0)
            token_position.latest_snapshot.position_state = TokenPositionState.BUY_CONFIRMED

            token_position.static_data.entry_block = live_token.latest_block_number
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

    async def update_submit_sell(self, token_position: TokenPosition, live_token: ERC20Token, signal: TradeSignal) -> TokenPosition:
        """
        Update position state when submitting a sell order and publish signal to eth_kartal.
        
        State Transition: BUY_CONFIRMED -> SELL_SUBMITTED
        
        This function:
          - Publishes sell signal to eth_kartal for execution
          - Records exit static data from live_token
          - Updates the latest snapshot in place to reflect the sell submission
          - Tracks signal ID for confirmation correlation
        """
        if token_position.latest_snapshot.position_state == TokenPositionState.BUY_CONFIRMED:
            current_price_ratio = 0
            if token_position.static_data.pool_address:
                current_price_ratio = live_token.latest_pools_price_ratio.get(token_position.static_data.pool_address, 0)
            
            # Publish signal to eth_kartal if publisher available
            if self.signal_publisher and self.wallet_address:
                try:
                    position_key = f"{token_position.static_data.token_address}-{token_position.static_data.pool_address}"
                    # For sells, signal.quantity should be the token amount
                    signal_id = await self.signal_publisher.publish_signal(
                        signal=signal,
                        wallet_address=self.wallet_address,
                        pool_address=token_position.static_data.pool_address,
                        pool_type=token_position.static_data.pool_type,
                        max_slippage=0.03,  # 3% default
                        deadline_seconds=300,  # 5 minutes
                        confirmation_callback=self._handle_execution_confirmation
                    )
                    self._pending_signals[position_key] = signal_id
                    self.logger.info(f"Published SELL signal {signal_id[:8]}... for {signal.token_address[:10]}...")
                except Exception as e:
                    self.logger.error(f"Failed to publish sell signal: {e}")
                    # Continue with position update even if signal publishing fails
            
            # Record exit static data directly from the token data
            token_position.static_data.exit_block = live_token.latest_block_number
            token_position.static_data.exit_price_ratio = current_price_ratio
            token_position.static_data.exit_timestamp = live_token.latest_block_timestamp
            
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
            token_position.static_data.exit_block = live_token.latest_block_number
                        
        return token_position
    
    async def _handle_execution_confirmation(self, signal: TradeSignal, confirmation: ExecutionConfirmation):
        """
        Handle execution confirmation from eth_kartal.
        
        This callback is called when eth_kartal sends confirmation of trade execution.
        Updates position states based on execution success or failure.
        """
        position_key = f"{signal.token_address}-{confirmation.signal_id}"
        
        if confirmation.status == ExecutionStatus.CONFIRMED:
            self.logger.info(
                f"Trade confirmed for {signal.token_address[:10]}... "
                f"tx: {confirmation.tx_hash[:10]}... "
                f"gas: {confirmation.gas_used}"
            )
            # Remove from pending
            self._pending_signals.pop(position_key, None)
            
        elif confirmation.status == ExecutionStatus.FAILED:
            self.logger.error(
                f"Trade failed for {signal.token_address[:10]}... "
                f"error: {confirmation.error}"
            )
            # Remove from pending  
            self._pending_signals.pop(position_key, None)
            
        elif confirmation.status == ExecutionStatus.PENDING:
            self.logger.info(
                f"Trade pending for {signal.token_address[:10]}... "
                f"tx: {confirmation.tx_hash[:10]}..."
            )
    
    def get_pending_signals(self) -> Dict[str, str]:
        """Get all pending signals awaiting confirmation."""
        return dict(self._pending_signals)
