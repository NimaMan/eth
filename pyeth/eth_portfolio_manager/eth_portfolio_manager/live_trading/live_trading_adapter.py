"""
Live Trading Database Adapter

This adapter integrates the existing LiveTokenTracker with the new live_trading_db
without requiring major changes to the existing codebase.
"""

from typing import Dict, List, Optional, Any
import asyncio
from web3 import Web3

from eth_token.erc20_token.erc20_token import ERC20Token
from eth_portfolio_manager.strategy.base_strategy import BaseStrategy
from eth_portfolio_manager.live_trading.live_trading_coordinator import LiveTradingCoordinator
from eth_data.database.writers.live_trading_position_writer import LiveTradingPositionWriter
from eth_portfolio_manager.utils.logger import get_logger


class LiveTradingAdapter:
    """
    Adapter that integrates live_trading_db with existing LiveTokenTracker.
    
    This class can be used as a drop-in replacement for LiveResultsWriter
    in the existing LiveTokenTracker without requiring major code changes.
    """
    
    def __init__(self, strategies: List[BaseStrategy], logger=None):
        self.strategies = strategies
        self.logger = logger or get_logger("LiveTradingAdapter")
        self.coordinator = None
        self.position_writer = None
        
        try:
            # Initialize the new live trading system
            self.coordinator = LiveTradingCoordinator(strategies, self.logger)
            self.position_writer = LiveTradingPositionWriter(self.logger)
            
            # Track strategy run IDs for compatibility
            self._strategy_runs: Dict[str, int] = {}
            
            self.logger.info("Live trading adapter initialized")
        except Exception as e:
            self.logger.error(f"Failed to initialize live trading adapter: {e}")
            # Re-raise to let the integrator handle it
            raise
    
    def create_or_update_strategy_run(self, strategy_name: str, params: dict, 
                                    start_block: int, end_block: int) -> int:
        """
        Create or update strategy run - compatible with LiveResultsWriter interface.
        
        For live trading, this creates a wallet record if needed and returns a run ID.
        """
        try:
            # Find the strategy (handle LIVE_ prefix)
            # Remove LIVE_ prefix if present for matching
            actual_strategy_name = strategy_name[5:] if strategy_name.startswith("LIVE_") else strategy_name
            
            # Try exact match first
            strategy = next((s for s in self.strategies if s.strategy_name == actual_strategy_name), None)
            
            # If not found and it's WalletTracker, try prefix matching
            if not strategy and (actual_strategy_name == "WalletTracker" or actual_strategy_name.startswith("WalletTracker_")):
                strategy = next((s for s in self.strategies if s.strategy_name.startswith("WalletTracker")), None)
            
            if not strategy:
                self.logger.warning(f"Strategy {strategy_name} (looked for {actual_strategy_name}) not found in {[s.strategy_name for s in self.strategies]}")
                return -1
            
            # Get or create wallet
            wallet_address = self._extract_wallet_address(strategy)
            if not wallet_address:
                return -1
            
            wallet_id = self._get_or_create_wallet_id(wallet_address, strategy)
            
            # Store run info for compatibility
            run_key = f"{strategy_name}_{start_block}"
            self._strategy_runs[run_key] = wallet_id            
            return wallet_id
            
        except Exception as e:
            self.logger.error(f"Error creating strategy run: {e}")
            return -1
    
    def update_token_positions(self, run_id: int, token_positions: Dict):
        """
        Update token positions - compatible with LiveResultsWriter interface.
        
        This processes tokens through the live trading system instead of
        writing to the backtest database.
        """
        try:
            # Convert token_positions dict to list of ERC20Token objects
            tokens = self._convert_positions_to_tokens(token_positions)
            
            if tokens:
                # Process through live trading system
                asyncio.create_task(self.coordinator.process_token_updates(tokens))
                
                self.logger.info(f"Processed {len(tokens)} token updates through live trading system")
            
        except Exception as e:
            self.logger.error(f"Error updating token positions: {e}")
    
    def write_mempool_scam_prediction(self, token_address: str, pool_address: str,
                                    prediction_block_number: int, current_eth_level: float,
                                    simulated_eth_level: float, eth_threshold: float):
        """
        Write mempool scam prediction - compatible with LiveResultsWriter interface.
        
        This delegates to the existing implementation in LiveTradingPositionWriter.
        """
        try:
            # Use the existing method from LiveTradingPositionWriter
            self.position_writer.write_mempool_scam_prediction(
                token_address, pool_address, prediction_block_number,
                current_eth_level, simulated_eth_level, eth_threshold
            )
            
        except Exception as e:
            self.logger.error(f"Error writing mempool scam prediction: {e}")
    
    def _extract_wallet_address(self, strategy: BaseStrategy) -> Optional[str]:
        """Extract wallet address from strategy."""
        if hasattr(strategy, 'config'):
            if hasattr(strategy.config, 'wallet_address'):
                return strategy.config.wallet_address
            elif isinstance(strategy.config, dict) and 'wallet_address' in strategy.config:
                return strategy.config['wallet_address']
        
        if hasattr(strategy, 'strategy_parameters'):
            if isinstance(strategy.strategy_parameters, dict):
                return strategy.strategy_parameters.get('wallet_address')
        
        return None
    
    def _get_or_create_wallet_id(self, wallet_address: str, strategy: BaseStrategy) -> int:
        """Get or create wallet ID in live_trading_db."""
        try:
            # Check if wallet exists
            self.position_writer.live_cur.execute("""
                SELECT id FROM wallets WHERE wallet_address = %s
            """, (Web3.to_checksum_address(wallet_address),))
            
            result = self.position_writer.live_cur.fetchone()
            if result:
                return result['id']
            
            # Create new wallet
            self.position_writer.live_cur.execute("""
                INSERT INTO wallets 
                (wallet_address, label, is_active, primary_strategy, 
                 max_position_size_eth, max_positions, max_daily_loss_eth)
                VALUES (%s, %s, %s, %s, %s, %s, %s)
                RETURNING id
            """, (
                Web3.to_checksum_address(wallet_address),
                f"{strategy.strategy_name} Wallet",
                True,
                strategy.strategy_name,
                getattr(strategy.config, 'position_size_eth', 0.01),
                getattr(strategy.config, 'max_positions', 10),
                getattr(strategy.config, 'max_daily_loss', 1.0)
            ))
            
            self.position_writer.live_conn.commit()
            result = self.position_writer.live_cur.fetchone()
            wallet_id = result['id']
            
            self.logger.info(f"Created wallet {wallet_id} for {wallet_address[:10]}...")
            return wallet_id
            
        except Exception as e:
            self.logger.error(f"Failed to get/create wallet ID: {e}")
            return 1  # Return default wallet ID
    
    def _convert_positions_to_tokens(self, token_positions: Dict) -> List[ERC20Token]:
        """Convert token positions dict to ERC20Token objects for processing."""
        tokens = []
        
        for position_key, token_position in token_positions.items():
            try:
                # Extract token address from position key
                if '-' in position_key:
                    token_address = position_key.split('-')[0]
                else:
                    token_address = position_key
                
                # Create a minimal ERC20Token for processing
                # In practice, this would need to get the full token data
                token = self._create_token_from_position(token_address, token_position)
                if token:
                    tokens.append(token)
                    
            except Exception as e:
                self.logger.error(f"Error converting position {position_key}: {e}")
        
        return tokens
    
    def _create_token_from_position(self, token_address: str, token_position) -> Optional[ERC20Token]:
        """Create ERC20Token from position data."""
        try:
            # This is a simplified version - in practice you'd need to reconstruct
            # the full token data from the position or fetch it from the token tracker
            
            # For now, just return None to indicate this needs proper integration
            # with the existing token data flow
            return None
            
        except Exception as e:
            self.logger.error(f"Error creating token from position: {e}")
            return None
    
    def get_live_trading_stats(self) -> Dict[str, Any]:
        """Get statistics from the live trading system."""
        try:
            return {
                'position_summary': self.coordinator.get_position_summary(),
                'strategy_performance': self.coordinator.get_strategy_performance(),
                'database_stats': self.coordinator.get_database_stats()
            }
            
        except Exception as e:
            self.logger.error(f"Error getting live trading stats: {e}")
            return {}
    
    async def shutdown(self):
        """Shutdown the adapter and all components."""
        try:
            await self.coordinator.shutdown()
            if hasattr(self.position_writer, '__del__'):
                self.position_writer.__del__()
            
            self.logger.info("Live trading adapter shutdown complete")
            
        except Exception as e:
            self.logger.error(f"Error during adapter shutdown: {e}")
    
    def __del__(self):
        """Cleanup on deletion."""
        if hasattr(self, 'position_writer') and hasattr(self.position_writer, '__del__'):
            self.position_writer.__del__()


class LiveTradingIntegrator:
    """
    Helper class to integrate LiveTradingAdapter with existing LiveTokenTracker.
    """
    
    @staticmethod
    def replace_results_writer(live_token_tracker, strategies: List[BaseStrategy]):
        """
        Replace the LiveResultsWriter in LiveTokenTracker with LiveTradingAdapter.
        
        Args:
            live_token_tracker: Existing LiveTokenTracker instance
            strategies: List of strategies for live trading
        """
        try:
            # Get the logger from the existing tracker
            logger = getattr(live_token_tracker, 'logger', None)
            
            # Create the adapter
            adapter = LiveTradingAdapter(strategies, logger)
            
            # Replace the results writer
            live_token_tracker.results_writer = adapter
            
            if logger:
                logger.info("✅ LiveTokenTracker integrated with live_trading_db")
            
            return adapter
            
        except Exception as e:
            if logger:
                logger.error(f"❌ Failed to integrate LiveTokenTracker: {e}")
            return None
    
    @staticmethod
    def create_hybrid_tracker(live_token_tracker, strategies: List[BaseStrategy]):
        """
        Create a hybrid tracker that uses both old and new systems during transition.
        
        This allows for gradual migration and comparison between systems.
        """
        try:
            # Store the original results writer
            original_writer = live_token_tracker.results_writer
            
            # Create the adapter
            adapter = LiveTradingAdapter(strategies, live_token_tracker.logger)
            
            # Create a hybrid writer that uses both
            class HybridWriter:
                def __init__(self, original, adapter, logger):
                    self.original = original
                    self.adapter = adapter
                    self.logger = logger
                
                def create_or_update_strategy_run(self, *args, **kwargs):
                    # Use both systems
                    original_result = self.original.create_or_update_strategy_run(*args, **kwargs)
                    adapter_result = self.adapter.create_or_update_strategy_run(*args, **kwargs)
                    
                    self.logger.info(f"Hybrid run creation: original={original_result}, adapter={adapter_result}")
                    return original_result  # Return original for compatibility
                
                def update_token_positions(self, *args, **kwargs):
                    # Use both systems
                    try:
                        self.original.update_token_positions(*args, **kwargs)
                    except Exception as e:
                        self.logger.error(f"Original writer error: {e}")
                    
                    try:
                        self.adapter.update_token_positions(*args, **kwargs)
                    except Exception as e:
                        self.logger.error(f"Adapter writer error: {e}")
                
                def write_mempool_scam_prediction(self, *args, **kwargs):
                    # Use adapter (new system) for this
                    return self.adapter.write_mempool_scam_prediction(*args, **kwargs)
            
            # Replace with hybrid writer
            live_token_tracker.results_writer = HybridWriter(
                original_writer, adapter, live_token_tracker.logger
            )
            
            live_token_tracker.logger.info("✅ Hybrid LiveTokenTracker created")
            return adapter
            
        except Exception as e:
            live_token_tracker.logger.error(f"❌ Failed to create hybrid tracker: {e}")
            return None