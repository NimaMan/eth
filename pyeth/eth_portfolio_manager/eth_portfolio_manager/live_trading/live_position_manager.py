"""
Live Position Manager

Manages live trading positions with proper database integration.
Coordinates between strategy decisions, signal publishing, and database tracking.
"""

from typing import Dict, Optional, Any
import asyncio
from uuid import UUID
from web3 import Web3

from eth_token.erc20_token.erc20_token import ERC20Token
from eth_portfolio_manager.core.data_models import TradeSignal, TradingDecision
from eth_portfolio_manager.core.token_position import TokenPosition
from eth_portfolio_manager.publishers.trade_signal_publisher import TradeSignalPublisher, ExecutionConfirmation, ExecutionStatus
from eth_portfolio_manager.live_trading.live_strategy_engine import LiveStrategyEngine
from eth_data.database.writers.live_trading_position_writer import LiveTradingPositionWriter
from eth_portfolio_manager.utils.logger import get_logger


class LivePositionManager:
    """
    Manages live positions with proper database integration.
    
    Responsibilities:
    1. Process token updates through strategy engines
    2. Create positions in live_trading_db when signals are published
    3. Track signal-position correlations
    4. Update position states based on execution confirmations
    """
    
    def __init__(self, wallet_id: int, strategy_engine: LiveStrategyEngine,
                 position_writer: LiveTradingPositionWriter,
                 signal_publisher: Optional[TradeSignalPublisher] = None):
        self.wallet_id = wallet_id
        self.strategy_engine = strategy_engine
        self.position_writer = position_writer
        self.signal_publisher = signal_publisher
        self.logger = get_logger(f"LivePositionManager-{strategy_engine.strategy_name}")
        
        # Track active positions: token_address -> position_data
        self._active_positions: Dict[str, Dict[str, Any]] = {}
        
        # Track signal-position correlations: signal_id -> position_id
        self._signal_position_map: Dict[str, int] = {}
        
        # Set execution confirmation callback if signal publisher available
        # TODO: Implement set_confirmation_callback in TradeSignalPublisher
        # if self.signal_publisher:
        #     self.signal_publisher.set_confirmation_callback(self._handle_execution_confirmation)
    
    async def process_token_update(self, token: ERC20Token) -> Optional[TokenPosition]:
        """
        Process token update through strategy and manage live positions.
        
        Args:
            token: Updated token data
            
        Returns:
            Updated TokenPosition if processed, None otherwise
        """
        try:
            token_address = Web3.to_checksum_address(token.token_address)
            
            # Get or create token position
            token_position = await self._get_or_create_token_position(token)
            if not token_position:
                return None
            
            # Process through strategy engine
            updated_position = await self.strategy_engine.process_updated_token(token, token_position)
            
            # Update position metrics if active
            if token_address in self._active_positions:
                await self._update_position_metrics(token_address, token)
            
            return updated_position
            
        except Exception as e:
            self.logger.error(f"Error processing token update {token.token_address[:10]}...: {e}")
            return None
    
    async def _get_or_create_token_position(self, token: ERC20Token) -> Optional[TokenPosition]:
        """Get existing TokenPosition or create new one for strategy processing."""
        try:
            token_address = Web3.to_checksum_address(token.token_address)
            
            # Check if we already have an active position
            if token_address in self._active_positions:
                # Convert back to TokenPosition for strategy processing
                return self._create_token_position_from_data(token, self._active_positions[token_address])
            
            # Create new TokenPosition for strategy analysis
            token_position = TokenPosition()
            token_position.update_from_token_data(token)
            
            return token_position
            
        except Exception as e:
            self.logger.error(f"Error getting/creating token position: {e}")
            return None
    
    def _create_token_position_from_data(self, token: ERC20Token, position_data: Dict[str, Any]) -> TokenPosition:
        """Create TokenPosition object from database position data."""
        token_position = TokenPosition()
        token_position.update_from_token_data(token)
        
        # Update with position-specific data
        if position_data.get('entry_price'):
            token_position.static_data.entry_price_ratio = float(position_data['entry_price'])
        if position_data.get('quantity_eth'):
            token_position.static_data.purchase_value = float(position_data['quantity_eth'])
        
        return token_position
    
    async def create_live_position(self, token: ERC20Token, signal: TradeSignal) -> Optional[int]:
        """
        Create a position in live_trading_db when a signal is published.
        
        Args:
            token: Token being traded
            signal: Trade signal being executed
            
        Returns:
            Position ID if successful, None otherwise
        """
        try:
            token_address = Web3.to_checksum_address(token.token_address)
            
            # Get pool information
            pool_addresses = getattr(token.token_data, 'pool_addresses', [])
            pool_address = pool_addresses[0] if pool_addresses else None
            
            if not pool_address:
                self.logger.warning(f"No pool address found for token {token_address[:10]}...")
                return None
            
            # Create position in database
            position_id = self.position_writer.create_position(
                wallet_id=self.wallet_id,
                token_address=token_address,
                pool_address=pool_address,
                strategy_name=self.strategy_engine.strategy_name,
                entry_signal_id=signal.signal_id if hasattr(signal, 'signal_id') else None,
                quantity_eth=signal.quantity if signal.decision == TradingDecision.SUBMIT_BUY else None
            )
            
            if position_id:
                # Track position locally
                self._active_positions[token_address] = {
                    'position_id': position_id,
                    'pool_address': pool_address,
                    'state': 'INIT',
                    'entry_signal_id': getattr(signal, 'signal_id', None),
                    'quantity_eth': signal.quantity if signal.decision == TradingDecision.SUBMIT_BUY else None
                }
                
                # Track signal-position correlation
                if hasattr(signal, 'signal_id'):
                    self._signal_position_map[str(signal.signal_id)] = position_id
                
                self.logger.info(f"Created live position {position_id} for {token_address[:10]}...")
                
            return position_id
            
        except Exception as e:
            self.logger.error(f"Failed to create live position: {e}")
            return None
    
    async def _handle_execution_confirmation(self, signal: TradeSignal, confirmation: ExecutionConfirmation):
        """
        Handle execution confirmation from eth_kartal.
        Updates position states based on execution results.
        """
        try:
            signal_id = str(confirmation.signal_id)
            position_id = self._signal_position_map.get(signal_id)
            
            if not position_id:
                self.logger.warning(f"No position found for signal {signal_id[:8]}...")
                return
            
            if confirmation.status == ExecutionStatus.CONFIRMED:
                # Update position state based on signal type
                if signal.decision == TradingDecision.SUBMIT_BUY:
                    success = self.position_writer.update_position_state(
                        position_id, 'BUY_CONFIRMED',
                        {
                            'entry_tx_hash': confirmation.tx_hash,
                            'entry_block': confirmation.block_number,
                            'entry_price': confirmation.actual_price,
                            'quantity_tokens': confirmation.actual_amount_tokens
                        }
                    )
                    if success:
                        # Update local tracking
                        token_address = signal.token_address
                        if token_address in self._active_positions:
                            self._active_positions[token_address]['state'] = 'BUY_CONFIRMED'
                
                elif signal.decision == TradingDecision.SUBMIT_SELL:
                    success = self.position_writer.update_position_state(
                        position_id, 'SELL_CONFIRMED',
                        {
                            'exit_tx_hash': confirmation.tx_hash,
                            'exit_block': confirmation.block_number,
                            'exit_price': confirmation.actual_price,
                            'realized_profit_eth': confirmation.realized_profit
                        }
                    )
                    if success:
                        # Remove from active positions (position closed)
                        token_address = signal.token_address
                        self._active_positions.pop(token_address, None)
                        self._signal_position_map.pop(signal_id, None)
                
                self.logger.info(
                    f"Position {position_id} confirmed: {signal.decision.name} "
                    f"tx: {confirmation.tx_hash[:10]}..."
                )
                
            elif confirmation.status == ExecutionStatus.FAILED:
                # Revert position state or mark as failed
                self.position_writer.update_position_state(
                    position_id, 'FAILED',
                    {'error_message': confirmation.error}
                )
                
                # Clean up tracking
                token_address = signal.token_address
                self._active_positions.pop(token_address, None)
                self._signal_position_map.pop(signal_id, None)
                
                self.logger.error(
                    f"Position {position_id} failed: {confirmation.error}"
                )
                
        except Exception as e:
            self.logger.error(f"Error handling execution confirmation: {e}")
    
    async def _update_position_metrics(self, token_address: str, token: ERC20Token):
        """Update position metrics (current price, unrealized profit) for active positions."""
        try:
            position_data = self._active_positions.get(token_address)
            if not position_data or position_data['state'] != 'BUY_CONFIRMED':
                return
            
            # Get current price from token
            pool_address = position_data['pool_address']
            current_price = 0
            if pool_address and hasattr(token.token_data, 'latest_pools_price_ratio'):
                current_price = token.token_data.latest_pools_price_ratio.get(pool_address, 0)
            
            if current_price > 0 and position_data.get('quantity_eth'):
                # Calculate current value
                quantity_eth = float(position_data['quantity_eth'])
                current_value_eth = quantity_eth * current_price
                
                # Update position metrics
                self.position_writer.update_position_metrics(
                    position_data['position_id'],
                    current_price,
                    current_value_eth
                )
                
        except Exception as e:
            self.logger.error(f"Error updating position metrics: {e}")
    
    def get_active_positions(self) -> Dict[str, Dict[str, Any]]:
        """Get all active positions."""
        return dict(self._active_positions)
    
    def get_position_count(self) -> int:
        """Get count of active positions."""
        return len(self._active_positions)