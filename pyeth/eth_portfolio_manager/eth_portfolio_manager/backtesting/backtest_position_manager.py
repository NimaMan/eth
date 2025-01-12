"""
Backtest Portfolio Position Manager

Objective:
---------
1. Simulate historical portfolio positions
2. Process historical trading signals
3. Track simulated portfolio state
4. Calculate historical metrics
5. No actual trades or Redis persistence needed

Key Differences from Live Manager:
-------------------------------
1. No Redis persistence
2. Simulated trade execution
3. Historical state tracking
4. Performance optimization for backtesting
5. Additional metrics for strategy analysis
"""

import asyncio
from typing import Dict, List, Optional
from datetime import datetime
from collections import OrderedDict

from eth_portfolio_manager.state.portfolio_position_manager import PortfolioPositionManager
from eth_portfolio_manager.backtesting.backtest_token_position_manager import TokenPositionManagerBacktest
from eth_portfolio_manager.core.data_models import TokenPositionData, TradeSignal


class BacktestPositionManager(PortfolioPositionManager):
    def __init__(self, logger=None):
        super().__init__(logger=logger)
        self.logger = logger
        self.positions: Dict[str, TokenPositionData] = {}
        self.token_position_manager = TokenPositionManagerBacktest()
        # Backtest specific tracking
        self.position_history = OrderedDict()
        
    async def update_token_positions(self, updated_tokens) -> Dict[str, TokenPositionData]:
        """Process token updates in backtest mode"""
        try:
            # Process token updates in parallel
            update_tasks = []
            for token_address, token in updated_tokens.items():
                task = self._process_single_token(token)
                update_tasks.append(task)
            
            # Wait for all updates to complete
            position_updates = await asyncio.gather(*update_tasks)
            
            # Record updates for analysis
            updated_positions = {
                addr: pos for addr, pos in position_updates if pos is not None
            }
            
            if updated_positions:
                await self._record_position_updates(updated_positions)
                await self.metrics_calculator.update_metrics(updated_positions)
                
            return updated_positions
            
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error updating backtest positions: {e}")
            raise
            
    async def _record_position_updates(self, positions: Dict[str, TokenPositionData]):
        """Record position updates for backtest analysis"""
        self.position_history[datetime.now()] = positions.copy() 