"""
TokenCacheService with batch processing and async queue

Objective:
---------
1. Provide distributed access to token data
2. Ensure data consistency across services
3. Support high-throughput token updates
4. Enable efficient querying of token states

Architecture:
-----------
1. Redis Backend
   - Hash storage for token data
   - Sorted sets for access patterns
   - Pub/sub for real-time updates

2. Data Management
   - Serialization/deserialization
   - Cache invalidation
   - Atomic operations
"""

import asyncio
import time
import redis
from typing import Dict, Optional
import orjson
from eth_token_monitor.utils.logger import get_logger
from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token


class TokenCacheService:
    def __init__(
        self,
        redis_url: str = "redis://localhost:6379/0",
        batch_size: int = 100,
        flush_interval: float = 60.0,
        logger=None
    ):
        self.redis = redis.Redis.from_url(redis_url, decode_responses=True)
        self.logger = logger or get_logger(name="token_cache", log_folder="tokens_live")
        
        # Batch processing configuration
        self.batch_size = batch_size
        self.flush_interval = flush_interval
        
        # Update queue and batching
        self._update_queue = asyncio.Queue()
        self._pending_updates: Dict[str, LiveERC20Token] = {}
        self._last_flush = time.time()
        
        # Start background tasks
        self._start_background_tasks()
        
    def _start_background_tasks(self):
        """Start background processing tasks"""
        asyncio.create_task(self._process_update_queue())
        asyncio.create_task(self._periodic_flush())
        
    async def store_token(self, token: LiveERC20Token) -> bool:
        """Queue token update for batch processing"""
        try:
            await self._update_queue.put(token)
            return True
        except Exception as e:
            self.logger.error(f"Error queueing token update {token.contract_address}: {e}")
            return False
            
    async def _process_update_queue(self):
        """Process token updates in batches"""
        while True:
            try:
                while len(self._pending_updates) < self.batch_size:
                    try:
                        token = await asyncio.wait_for(
                            self._update_queue.get(),
                            timeout=0.1
                        )
                        self._pending_updates[token.contract_address] = token
                    except asyncio.TimeoutError:
                        break
                        
                if self._pending_updates:
                    await self._flush_updates()
                    
            except Exception as e:
                self.logger.error(f"Error in update queue processing: {e}")
                await asyncio.sleep(1)
                
    async def _periodic_flush(self):
        """Periodically flush pending updates"""
        while True:
            try:
                await asyncio.sleep(self.flush_interval)
                if self._pending_updates:
                    await self._flush_updates()
            except Exception as e:
                self.logger.error(f"Error in periodic flush: {e}")
                
    async def _flush_updates(self):
        """Flush pending updates to Redis"""
        if not self._pending_updates:
            return
            
        try:
            pipeline = self.redis.pipeline()
            
            for address, token in self._pending_updates.items():
                token_data = token.to_dict()
                pipeline.hset(f"token:{address}", mapping=token_data)
                pipeline.sadd("cached_tokens", address)  # Add to cached set
            
            await pipeline.execute()
            self._pending_updates.clear()
            self._last_flush = time.time()
            
        except Exception as e:
            self.logger.error(f"Error flushing updates to Redis: {e}")

    async def store_tokens_batch(self, tokens: Dict[str, LiveERC20Token]) -> bool:
        """Store multiple tokens at once"""
        try:
            pipeline = self.redis.pipeline()
            
            for addr, token in tokens.items():
                token_data = token.to_dict()
                pipeline.hset(f"token:{addr}", mapping=token_data)
                pipeline.sadd("cached_tokens", addr)
                
            await pipeline.execute()
            return True
            
        except Exception as e:
            self.logger.error(f"Error storing token batch: {e}")
            return False

    async def get_cached_tokens(self) -> Dict[str, dict]:
        """Get all cached tokens from Redis"""
        try:
            cached_addrs = self.redis.smembers("cached_tokens")
            cached_tokens = {}
            
            for addr in cached_addrs:
                token_data = self.redis.hgetall(f"token:{addr}")
                if token_data:
                    cached_tokens[addr] = token_data
                    
            return cached_tokens
            
        except Exception as e:
            self.logger.error(f"Error getting cached tokens: {e}")
            return {}

    def __getitem__(self, contract_address: str) -> Optional[dict]:
        """Get token data by contract address"""
        try:
            token_data = self.redis.hgetall(f"token:{contract_address}")
            return token_data if token_data else None
        except Exception as e:
            self.logger.error(f"Error getting token {contract_address}: {e}")
            return None 