import asyncio
from typing import Dict, Optional

from eth_portfolio_manager.core.portfolio_position_manager import PortfolioPositionManager
from eth_portfolio_manager.core.data_models import TokenPositionData
from eth_portfolio_manager.live.live_portfolio_state_server import PortfolioStateServer
from eth_portfolio_manager.live.live_token_position_manager import LiveTokenPositionManager
from eth_portfolio_manager.strategy.base import BaseStrategy


class LivePortfolioPositionManager(PortfolioPositionManager):
    def __init__(self, investment_strategy: BaseStrategy, logger=None):
        super().__init__(logger=logger)
        self.state_server = PortfolioStateServer(logger=self.logger)
        self.positions: Dict[str, TokenPositionData] = self.state_server.current_positions
        self.token_position_manager = LiveTokenPositionManager(investment_strategy=investment_strategy)
        
    async def initialize(self):
        """Initialize portfolio state from Redis"""
        try:
            await self.state_server.load_state()
            self.positions = self.state_server.current_positions
            await self.get_portfolio_metrics()
            self.logger.info(f"Initialized portfolio with {len(self.positions)} positions")
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error initializing portfolio: {e}")
            raise e

    async def update_portfolio_tokens_positions(self, updated_tokens) -> Dict[str, TokenPositionData]:
        """
        Process multiple token updates concurrently and update portfolio state
        
        Args:
            updated_tokens: Dict mapping token addresses to their updated data
            
        Returns:
            Dict of updated position data
            
        Implementation:
        1. Group tokens into batches for efficient processing
        2. Process each batch concurrently using asyncio.gather
        3. Update Redis state atomically
        4. Update portfolio metrics
        """
        try:
            # Process token updates in parallel
            update_tasks = []
            for token_address, token in updated_tokens.items():
                task = self.process_single_token(token)
                update_tasks.append(task)
            
            # Wait for all updates to complete
            position_updates = await asyncio.gather(*update_tasks)
            
            # Combine updates into single dict
            updated_positions = {
                addr: pos for addr, pos in position_updates if pos is not None
            }
            
            # Batch update Redis state and metrics
            if updated_positions:
                await self.state_server.update_positions(updated_positions)
            return updated_positions
            
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error updating token positions: {e}")
            raise
    
    def get_position(self, token_address: str) -> Optional[TokenPositionData]:
        """Get position data for a specific token"""
        return self.positions.get(token_address)
   