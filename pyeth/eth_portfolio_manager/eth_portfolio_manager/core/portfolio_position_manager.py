"""
Portfolio Position Manager

Objective:
---------
1. Manage portfolio-wide position tracking
2. Process updates for each token concurrently
3. Limit concurrency to prevent overwhelming the system

Event Flow:
----------
- A dict of updated tokens from the blocks has been received with a dict of address: LiveERC20Token
   - New token detected if not in the current token positions dictionary
      - Source: PortfolioManager token creation events
      - Handler: create_position()
      - Action: Initializes new position tracking
   - The Token Position Manager has received a batch of updated tokens
      - Source: Updated tokens from the blocks
      - Handler: Token Position Manager process the updates and analyzes the strategy decision for each token
      - Action: Concurrent position processing
   - The Portfolio Position Manager has received the updated tokens from the Token Position Manager
      - Source: Token Position Manager
      - Handler: Portfolio Position Manager process the updates and updates the portfolio positions
      - Action: Concurrent position processing
"""

import asyncio
from typing import Dict, List, Optional, Tuple
from collections import defaultdict

from eth_portfolio_manager.core.token_position import TokenPosition
from eth_token.live_erc20_token.live_token import LiveERC20Token
from eth_portfolio_manager.utils.logger import get_logger


class PortfolioPositionManager:
   def __init__(self, token_position_manager, max_concurrency=20, logger=None):
      self.logger = logger or get_logger(name="portfolio_manager")
      self.token_positions: Dict[str, TokenPosition] = {}
      self.token_position_manager = token_position_manager
      self.semaphore = asyncio.Semaphore(value=max_concurrency)  # concurrency limit
   
   def create_position(self, live_token: LiveERC20Token) -> TokenPosition:
      """Create a new position for a token"""
      return TokenPosition.create_from_token(live_token)

   async def process_single_token(self, live_token: LiveERC20Token) -> Tuple[str, Optional[TokenPosition]]:
      """Process updates for a single token"""
      try:
         # Create or get existing position
         token_address = live_token.token_data.contract_address
         if token_address not in self.token_positions:
            token_position = self.create_position(live_token) 
            self.token_positions[token_address] = token_position
         else:
            token_position = self.token_positions[token_address]
         # Process token updates and update the position inplace
         # Update position state
         token_position.update_from_token_data(live_token)
         # Apply investment strategy to generate trade signals
         self.token_position_manager.process_updated_token(live_token, token_position)
         return token_address, token_position
         
      except Exception as e:
         self.logger.error(f"{self.__class__.__name__} Error processing token {token_address} with strategy {self.token_position_manager.strategy_name}: {e}")
         return token_address, None
   
   async def update_portfolio_tokens_positions(self, updated_tokens: Dict[str, LiveERC20Token]) -> Dict[str, TokenPosition]:
         """Process token updates in backtest mode"""
         update_tasks = []

         for live_token in updated_tokens.values():
            # Limit concurrency with semaphore
            async def sem_task(lt=live_token):
                async with self.semaphore:
                    return await self.process_single_token(lt)

            task = asyncio.create_task(sem_task())
            update_tasks.append(task)

         try:
            await asyncio.gather(*update_tasks)
         except asyncio.CancelledError:
            self.logger.info("Backtest run cancelled - performing cleanup.")
            # Optionally handle partial updates or cleanup
            raise
         except KeyboardInterrupt:
            self.logger.info("KeyboardInterrupt received - cancelling tasks.")
            for t in update_tasks:
                t.cancel()
            await asyncio.gather(*update_tasks, return_exceptions=True)
            # Optionally handle partial updates or cleanup
            raise

         # Return final token positions if needed
         return self.token_positions
                     
