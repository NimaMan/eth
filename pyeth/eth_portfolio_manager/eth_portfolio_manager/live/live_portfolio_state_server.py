"""
Portfolio State Server

Objective:
---------
1. Provide persistent storage for portfolio state using Redis
2. Track historical position data and portfolio metrics
3. Enable fast state recovery and external data access
4. Support concurrent position updates

Key Features:
------------
1. Position Storage:
   - Current positions with full state
   - Historical position data with timestamps
   - Position metrics and analytics

2. Portfolio Metrics:
   - Total portfolio value
   - Profit/Loss tracking
   - Position counts and statistics

3. State Management:
   - Async state updates
   - Atomic operations
   - Data consistency checks

4. External Access:
   - REST API endpoints
   - Real-time position data
   - Historical data queries
"""

import orjson
from datetime import datetime
from typing import Dict, List, Optional
import asyncio
import redis.asyncio as aioredis

from eth_portfolio_manager.core.token_position import TokenPosition
from eth_portfolio_manager.utils.logger import get_logger


class PortfolioStateServer:
    def __init__(self, redis_url: str = "redis://localhost:6379/0", logger=None):
        self.redis = aioredis.from_url(redis_url, decode_responses=True)
        self.logger = logger or get_logger(name="portfolio_manager")
        self.token_positions: Dict[str, TokenPosition] = {}
        
    async def load_state(self):
        """Load existing state from Redis"""
        try:
            # Load positions using async scan
            async for key in self.redis.scan_iter("position:*"):
                if ":history:" in key:
                    continue
                    
                pos_data = await self.redis.get(key)
                if pos_data:
                    pos_dict = orjson.loads(pos_data)
                    token_address = key.split(":")[-1]
                    self.current_positions[token_address] = TokenPosition(**pos_dict)
                    
            self.logger.info(f"Loaded {len(self.current_positions)} positions from Redis")
            
        except Exception as e:
            self.logger.error(f"Error loading state: {e}")
            raise
            
    async def update_positions(self, positions: Dict[str, TokenPositionData]):
        """
        Update multiple positions atomically
        
        Args:
            positions: Dict of token_address -> TokenPositionData
        """
        try:
            # Create pipeline for atomic updates
            async with self.redis.pipeline(transaction=True) as pipe:
                for token_address, position in positions.items():
                    # Update current position
                    position_key = f"position:{token_address}"
                    pipe.set(
                        position_key,
                        orjson.dumps(position.__dict__)
                    )
                    
                    # Add to history
                    history_key = f"position:history:{token_address}"
                    history_entry = {
                        "timestamp": datetime.now().isoformat(),
                        **position.__dict__
                    }
                    pipe.rpush(
                        history_key,
                        orjson.dumps(history_entry)
                    )
                    
                    # Update metrics
                    if position.has_active_position:
                        metrics_key = f"metrics:{token_address}"
                        pipe.hset(
                            metrics_key,
                            mapping={
                                "current_value": str(position.current_value),
                                "profit_loss": str(position.realized_profit + position.unrealized_profit),
                                "last_updated": datetime.now().isoformat()
                            }
                        )
                
                await pipe.execute()
                
            self.current_positions.update(positions)
            #self.logger.info(f"Updated {len(positions)} positions in Redis")
            
        except Exception as e:
            self.logger.error(f"Error updating positions: {e}")
            raise
            
    async def get_position_history(self, token_address: str, limit: int = 100) -> List[Dict]:
        """Get historical position data"""
        try:
            history_key = f"position:history:{token_address}"
            history_data = await self.redis.lrange(history_key, -limit, -1)
            return [orjson.loads(entry) for entry in history_data]
        except Exception as e:
            self.logger.error(f"Error getting position history: {e}")
            raise            
            
    async def clear_state(self):
        """Clear all state (useful for testing)"""
        try:
            async with self.redis.pipeline(transaction=True) as pipe:
                pipe.delete(*[f"position:{addr}" for addr in self.current_positions])
                pipe.delete(*[f"position:history:{addr}" for addr in self.current_positions])
                pipe.delete(*[f"metrics:{addr}" for addr in self.current_positions])
                await pipe.execute()
                
            self.current_positions.clear()
            self.logger.info("Cleared all state from Redis")
        except Exception as e:
            self.logger.error(f"Error clearing state: {e}")
            raise
            
    async def update_portfolio_metrics(self, metrics):
        """Update portfolio metrics in Redis"""
        try:
            await self.redis.hset("metrics", mapping=metrics.__dict__)
        except Exception as e:
            self.logger.error(f"Error updating portfolio metrics: {e}")
            raise