"""
Token Update Notifier - Broadcast which tokens changed in the latest block.

This component now publishes only token address lists via ZeroMQ;
consumers hydrate full state from Redis snapshots.
"""

import zmq
import zmq.asyncio
import json
import time
from typing import Optional
from eth_portfolio_manager.utils.logger import get_logger


class TokenUpdateNotifier:
    """
    Publishes token tracking updates to external consumers.

    This is a one-way, low-latency invalidation feed. Full token state and
    startup discovery live in Redis token snapshots and the token snapshot index.
    """
    
    def __init__(
        self,
        pub_endpoint: str = "tcp://*:5557",
        logger=None
    ):
        """Initialize the notifier."""
        self.logger = logger or get_logger("token_update_notifier")
        self.pub_endpoint = pub_endpoint
        
        # Reference to cache for data access
        self.cache = None
        
        # ZMQ context and sockets
        self._context: Optional[zmq.asyncio.Context] = None
        self._pub_socket: Optional[zmq.asyncio.Socket] = None
        
        # State
        self._is_running = False
        self._last_published_block = 0
    
    def set_cache(self, cache):
        """Set reference to the token update cache."""
        self.cache = cache
    
    async def start(self):
        """Start the publisher."""
        if self._is_running:
            return
        
        self.logger.info("Starting token update notifier")
        self._is_running = True
        
        try:
            # Initialize ZMQ
            self._context = zmq.asyncio.Context()
            
            # Set up PUB socket
            self._pub_socket = self._context.socket(zmq.PUB)
            self._pub_socket.bind(self.pub_endpoint)
            self.logger.info(f"PUB socket bound to {self.pub_endpoint}")
            
        except Exception as e:
            self.logger.error(f"Error starting publisher: {e}")
            await self.stop()
            raise
    
    async def stop(self):
        """Stop the publisher."""
        if not self._is_running:
            return
        
        self.logger.info("Stopping token update notifier")
        self._is_running = False
        
        # Close sockets
        if self._pub_socket:
            self._pub_socket.close()
        
        # Terminate context
        if self._context:
            self._context.term()
        
        self.logger.info("Token update notifier stopped")
    
    async def publish_all(self):
        """
        Publish all tokens from cache.
        Used at startup or for full resync.
        """
        if not self._is_running or not self._pub_socket or not self.cache:
            return
        
        try:
            token_addresses = await self.cache.get_all_token_addresses()
            
            if token_addresses:
                message = {
                    'type': 'full_update',
                    'timestamp': time.time(),
                    'token_count': len(token_addresses),
                    'data': token_addresses
                }
                
                json_data = json.dumps(message)
                await self._pub_socket.send_string(json_data)
                
                self.logger.info(f"Published full update with {len(token_addresses)} tokens")
            
        except Exception as e:
            self.logger.error(f"Error publishing full update: {e}")
    
    async def publish_block_updates(self, block_number: int):
        """
        Publish updates for a specific block.
        Used after each new block is processed.
        """
        if not self._is_running or not self._pub_socket or not self.cache:
            return
        
        try:
            updated_addresses = await self.cache.get_updated_token_addresses()
            
            if updated_addresses:
                message = {
                    'type': 'block_update',
                    'timestamp': time.time(),
                    'block_number': block_number,
                    'token_count': len(updated_addresses),
                    'data': updated_addresses
                }
                
                json_data = json.dumps(message)
                await self._pub_socket.send_string(json_data)
                
                self._last_published_block = block_number
                self.logger.debug(f"Published block {block_number} update with {len(updated_addresses)} updated tokens")
            else:
                self.logger.debug(f"No tokens updated in block {block_number}, skipping publish")
            
        except Exception as e:
            self.logger.error(f"Error publishing block update: {e}")
