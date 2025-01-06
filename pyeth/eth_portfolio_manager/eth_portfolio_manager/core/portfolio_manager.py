"""
Portfolio Manager

Objective:
---------
1. Monitor token updates from LiveTokenManager
2. Process position updates when new token data is available
3. Maintain portfolio state and metrics
"""

import time
from typing import Dict, Optional, Set
import asyncio
from datetime import datetime

from eth_portfolio_manager.state.portfolio_position_manager import PortfolioPositionManager
from eth_token_monitor.token_manager.live_token_manager import LiveTokenManager
from eth_portfolio_manager.utils.logger import get_logger


class PortfolioManager:
    def __init__(
        self,
        logger=None,
        warmup_blocks: int = 100,
    ):
        self.logger = logger or get_logger(name="portfolio_manager")
        self.token_manager = LiveTokenManager(logger=self.logger, warmup_blocks=warmup_blocks)
        self.portfolio_position_manager = PortfolioPositionManager(logger=self.logger)
        self._shutdown_event = asyncio.Event()

    async def _monitor_token_updates(self):
        """Monitor for token updates using event notification"""
        self.logger.info("=== Portfolio Monitor Task Starting ===")
        
        while not self._shutdown_event.is_set():
            try:
                # Wait for updates to be available
                await self.token_manager.new_updates_event.wait()
                    
                # Process all available updates in priority order
                while not self.token_manager.unprocessed_updates.empty():
                    block_num, token_updates = await self.token_manager.unprocessed_updates.get()
                    
                    # Update positions
                    start_time = time.time()
                    updated_positions = await self.portfolio_position_manager.update_token_positions(token_updates)
                    end_time = time.time()
                    
                    # Get latest metrics
                    metrics = await self.portfolio_position_manager.get_portfolio_metrics()
                    self.logger.info(f"Portfolio metrics after update in {end_time - start_time} seconds for block {block_num} and {len(updated_positions)}: {metrics}")
                    
                    # Mark task as done
                    self.token_manager.unprocessed_updates.task_done()
                
                # Clear the event because we've processed everything
                if self.token_manager.unprocessed_updates.empty():
                    self.token_manager.new_updates_event.clear()
                
            except Exception as e:
                self.logger.error(f"PortfolioManager: Error processing updates: {e}", exc_info=True)
                await asyncio.sleep(1)

    async def start(self):
        """Start the portfolio manager"""
        try:
            self.logger.info("=== Starting PortfolioManager ===")
            
            # Initialize components
            self.logger.info("Initializing portfolio position manager...")
            await self.portfolio_position_manager.initialize()
            
            # Start token manager (includes its own monitoring)
            self.logger.info("Starting token manager...")
            token_manager_task = asyncio.create_task(self.token_manager.start())
            
            # Start our monitoring
            self.logger.info("Starting portfolio monitoring...")
            self._monitor_task = asyncio.create_task(self._monitor_token_updates())
            
            # Keep running until shutdown
            while not self._shutdown_event.is_set():
                await asyncio.sleep(1)
                
                # Check token manager task
                if token_manager_task.done():
                    if token_manager_task.exception():
                        raise token_manager_task.exception()
                
                # Check monitor task
                if self._monitor_task.done():
                    if self._monitor_task.exception():
                        raise self._monitor_task.exception()
            
        except Exception as e:
            self.logger.error(f"Portfolio Manager startup error: {e}", exc_info=True)
            await self.stop()
            raise
        
    async def stop(self):
        """Stop the portfolio manager"""
        self._shutdown_event.set()
        await self.token_manager.stop()
        self.logger.info("Portfolio manager stopped")

   