"""
Portfolio Position Manager

Objective:
---------
1. Manage portfolio-wide position tracking
2. Process updates for each token concurrently

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

from eth_portfolio_manager.core.token_position import TokenPosition
from eth_token.erc20_token.erc20_token import ERC20Token
from eth_portfolio_manager.core.token_positions_cache import TokenPositionsCache
from eth_portfolio_manager.utils.logger import get_logger


class StrategyPositionManager:
   def __init__(self, strategy_engine, max_concurrency=20, logger=None, token_positions_cache_max_size=10000):
      self.logger = logger or get_logger(name="portfolio_manager")
      self.token_positions_cache = TokenPositionsCache(max_size=token_positions_cache_max_size)
      self.strategy_engine = strategy_engine
      self.semaphore = asyncio.Semaphore(value=max_concurrency)  # concurrency limit
   
   def create_position(self, live_token: ERC20Token) -> TokenPosition:
      """Create a new position for a token"""
      return TokenPosition.create_from_token(live_token)

   async def process_single_token(self, live_token: ERC20Token) -> Tuple[str, Dict[str, Optional[TokenPosition]]]:
      """
      Process updates for a single token for every pool where it is active.
      This method uses the composite key (token_address:pool_address) in the cache.
      """
      try:
         token_address = live_token.contract_address
         results: Dict[str, Optional[TokenPosition]] = {}
         
         # Assuming pool_addresses is a collection of one or more pool address strings.
         for pool_address in live_token.pool_addresses:
            token_position = self.token_positions_cache.get(token_address, pool_address)
            if token_position is None:
               token_position = self.create_position(live_token)
               self.token_positions_cache.add(token_position, token_address, pool_address)
            else:
               # Process token updates and update the position in place.
               token_position.update_from_token_data(live_token)
               self.strategy_engine.process_updated_token(live_token, token_position)
            
            results[pool_address] = token_position
         
         return token_address, results
         
      except Exception as e:
         self.logger.error(
            f"{self.__class__.__name__} Error processing token {token_address} with strategy {self.strategy_engine.strategy_name}: {e}"
         )
         # Return token address with an empty result if error occurs.
         return token_address, {}

   async def update_updated_tokens_positions(self, updated_tokens: Dict[str, ERC20Token]) -> Dict[str, Dict[str, TokenPosition]]:
         """
         Process token updates in backtest mode concurrently.
         Returns a mapping from token address to a dict mapping pool_address -> TokenPosition.
         """
         update_tasks = []

         for live_token in updated_tokens.values():
            # Limit concurrency with semaphore.
            async def sem_task(lt=live_token):
                async with self.semaphore:
                   return await self.process_single_token(lt)

            task = asyncio.create_task(sem_task())
            update_tasks.append(task)

         try:
            results = await asyncio.gather(*update_tasks)
         except asyncio.CancelledError:
            self.logger.info("Backtest run cancelled - performing cleanup.")
            raise
         except KeyboardInterrupt:
            self.logger.info("KeyboardInterrupt received - cancelling tasks.")
            for t in update_tasks:
                t.cancel()
            results = await asyncio.gather(*update_tasks, return_exceptions=True)
            raise

         # Aggregate results: token address -> (pool_address -> TokenPosition)
         updated_tokens_positions = {}
         for token_address, pos_dict in results:
             updated_tokens_positions[token_address] = pos_dict

         return updated_tokens_positions
                     
