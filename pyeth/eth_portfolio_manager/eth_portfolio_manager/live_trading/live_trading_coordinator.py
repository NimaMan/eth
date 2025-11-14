"""
Live Trading Coordinator

Coordinates between multiple components for live trading:
- Strategy execution via LivePositionManager
- Database operations via LiveTradingPositionWriter
- Signal publishing via TradeSignalPublisher
- Pool tracking and token monitoring
"""

from typing import Dict, List, Optional, Any
import asyncio
from web3 import Web3

from eth_token.erc20_token.erc20_token import ERC20Token
from eth_portfolio_manager.strategy.base_strategy import BaseStrategy
from eth_portfolio_manager.live_trading.live_strategy_engine import LiveStrategyEngine
from eth_portfolio_manager.live_trading.live_position_manager import LivePositionManager
from eth_portfolio_manager.notifications.trade_signal_publisher import TradeSignalPublisher
from eth_data.database.writers.live_trading_position_writer import LiveTradingPositionWriter


class LiveTradingCoordinator:
    """
    Coordinates live trading operations across multiple wallets and strategies.
    
    This class replaces the direct use of BacktestStrategyEngine in live trading
    and provides proper integration with live_trading_db.
    """
    
    def __init__(self, strategies: List[BaseStrategy], logger):
        self.strategies = strategies
        self.logger = logger
        
        # Initialize database writer
        self.position_writer = LiveTradingPositionWriter(self.logger)
        
        # Initialize signal publisher (optional - can be None for testing)
        self.signal_publisher = TradeSignalPublisher(self.logger)
        
        # Position managers by wallet_id
        self._position_managers: Dict[int, Dict[str, LivePositionManager]] = {}
        
        # Initialize position managers for each strategy
        self._initialize_position_managers()
    
    def _initialize_position_managers(self):
        """Initialize position managers for each strategy and wallet."""
        for strategy in self.strategies:
            # Get wallet address from strategy
            wallet_address = self._extract_wallet_address(strategy)
            if not wallet_address:
                self.logger.warning(f"No wallet address found for strategy {strategy.strategy_name}")
                continue
            
            # Convert to wallet_id (would need to get from database in production)
            wallet_id = self._get_or_create_wallet_id(wallet_address, strategy)
            
            # Create strategy engine
            strategy_engine = LiveStrategyEngine(strategy, self.signal_publisher)
            
            # Create position manager
            position_manager = LivePositionManager(
                wallet_id=wallet_id,
                strategy_engine=strategy_engine,
                position_writer=self.position_writer,
                signal_publisher=self.signal_publisher
            )
            
            # Store by wallet_id and strategy name
            if wallet_id not in self._position_managers:
                self._position_managers[wallet_id] = {}
            self._position_managers[wallet_id][strategy.strategy_name] = position_manager
            
            self.logger.info(
                f"Initialized position manager for {strategy.strategy_name} "
                f"wallet: {wallet_address[:10]}..."
            )
    
    def _extract_wallet_address(self, strategy: BaseStrategy) -> Optional[str]:
        """Extract wallet address from strategy configuration."""
        # Try different ways strategies might store wallet address
        if hasattr(strategy, 'config'):
            if hasattr(strategy.config, 'wallet_address'):
                return strategy.config.wallet_address
            elif isinstance(strategy.config, dict) and 'wallet_address' in strategy.config:
                return strategy.config['wallet_address']
        
        if hasattr(strategy, 'strategy_parameters'):
            if isinstance(strategy.strategy_parameters, dict):
                return strategy.strategy_parameters.get('wallet_address')
        
        if hasattr(strategy, 'wallet_address'):
            return strategy.wallet_address
        
        return None
    
    def _get_or_create_wallet_id(self, wallet_address: str, strategy: BaseStrategy) -> int:
        """Get or create wallet ID in live_trading_db."""
        try:
            # Check if wallet exists
            existing_wallets = self.position_writer.live_cur.execute("""
                SELECT id FROM wallets WHERE wallet_address = %s
            """, (Web3.to_checksum_address(wallet_address),))
            
            result = self.position_writer.live_cur.fetchone()
            if result:
                return result['id']
            
            # Create new wallet
            self.position_writer.live_cur.execute("""
                INSERT INTO wallets 
                (wallet_address, label, is_active, primary_strategy, max_position_size_eth, max_positions)
                VALUES (%s, %s, %s, %s, %s, %s)
                RETURNING id
            """, (
                Web3.to_checksum_address(wallet_address),
                f"{strategy.strategy_name} Wallet",
                True,
                strategy.strategy_name,
                getattr(strategy.config, 'position_size_eth', 0.01),
                getattr(strategy.config, 'max_positions', 10)
            ))
            
            self.position_writer.live_conn.commit()
            result = self.position_writer.live_cur.fetchone()
            wallet_id = result['id']
            
            self.logger.info(f"Created wallet {wallet_id} for {wallet_address[:10]}...")
            return wallet_id
            
        except Exception as e:
            self.logger.error(f"Failed to get/create wallet ID: {e}")
            # Return a default wallet_id (assumes wallet 1 exists)
            return 1
    
    async def process_token_updates(self, tokens: List[ERC20Token]) -> Dict[str, Any]:
        """
        Process token updates through all active strategies.
        
        Args:
            tokens: List of updated tokens
            
        Returns:
            Summary of processing results
        """
        results = {
            'tokens_processed': 0,
            'positions_updated': 0,
            'signals_published': 0,
            'errors': []
        }
        
        for token in tokens:
            try:
                # Skip scammed tokens
                if hasattr(token, 'is_scam') and token.is_scam:
                    continue
                
                token_results = await self._process_single_token(token)
                
                results['tokens_processed'] += 1
                results['positions_updated'] += token_results.get('positions_updated', 0)
                results['signals_published'] += token_results.get('signals_published', 0)
                
            except Exception as e:
                error_msg = f"Error processing token {token.token_address[:10]}...: {e}"
                self.logger.error(error_msg)
                results['errors'].append(error_msg)
        
        return results
    
    async def _process_single_token(self, token: ERC20Token) -> Dict[str, int]:
        """Process a single token through all position managers."""
        results = {'positions_updated': 0, 'signals_published': 0}
        
        # Process through each position manager
        for wallet_id, strategy_managers in self._position_managers.items():
            for strategy_name, position_manager in strategy_managers.items():
                try:
                    # Process token update
                    token_position = await position_manager.process_token_update(token)
                    if token_position:
                        results['positions_updated'] += 1
                    
                    # Check if any signals were published (would need integration with signal tracking)
                    # This would be tracked by the position manager internally
                    
                except Exception as e:
                    self.logger.error(
                        f"Error in {strategy_name} for {token.token_address[:10]}...: {e}"
                    )
        
        return results
    
    def get_position_summary(self) -> Dict[str, Any]:
        """Get summary of all active positions across all managers."""
        summary = {
            'total_active_positions': 0,
            'positions_by_strategy': {},
            'positions_by_wallet': {}
        }
        
        for wallet_id, strategy_managers in self._position_managers.items():
            wallet_positions = 0
            for strategy_name, position_manager in strategy_managers.items():
                strategy_positions = position_manager.get_position_count()
                
                summary['positions_by_strategy'][strategy_name] = strategy_positions
                summary['total_active_positions'] += strategy_positions
                wallet_positions += strategy_positions
            
            summary['positions_by_wallet'][f"wallet_{wallet_id}"] = wallet_positions
        
        return summary
    
    def get_strategy_performance(self) -> Dict[str, Dict[str, Any]]:
        """Get performance metrics for each strategy."""
        performance = {}
        
        for wallet_id, strategy_managers in self._position_managers.items():
            for strategy_name, position_manager in strategy_managers.items():
                # Get active positions
                active_positions = position_manager.get_active_positions()
                
                performance[strategy_name] = {
                    'wallet_id': wallet_id,
                    'active_positions': len(active_positions),
                    'strategy_engine': position_manager.strategy_engine.strategy_name,
                    'pending_signals': len(position_manager.strategy_engine.get_pending_signals())
                }
        
        return performance
    
    async def shutdown(self):
        """Gracefully shutdown the coordinator."""
        try:
            # Close database connections
            if hasattr(self.position_writer, '__del__'):
                self.position_writer.__del__()
            
            # Close signal publisher
            if self.signal_publisher:
                await self.signal_publisher.close()
            
            self.logger.info("Live trading coordinator shutdown complete")
            
        except Exception as e:
            self.logger.error(f"Error during shutdown: {e}")
    
    def get_database_stats(self) -> Dict[str, int]:
        """Get database statistics for monitoring."""
        try:
            # Get position counts
            self.position_writer.live_cur.execute("""
                SELECT 
                    COUNT(*) as total_positions,
                    COUNT(CASE WHEN is_active = true THEN 1 END) as active_positions,
                    COUNT(DISTINCT wallet_id) as unique_wallets,
                    COUNT(DISTINCT strategy_name) as unique_strategies
                FROM live_positions
            """)
            
            result = self.position_writer.live_cur.fetchone()
            return dict(result) if result else {}
            
        except Exception as e:
            self.logger.error(f"Error getting database stats: {e}")
            return {}
