"""
Token Tracking Publisher - Publish token state to external consumers

This component publishes token and pool information via ZeroMQ to
external consumers like the Rust mempool processor.
"""

import asyncio
import zmq
import zmq.asyncio
import json
import time
from typing import Dict, Any, Optional
from eth_portfolio_manager.utils.logger import get_logger


class TokenTrackingPublisher:
    """
    Publishes token tracking updates to external consumers.
    
    Provides:
    1. PUB/SUB interface for streaming updates
    2. REQ/REP interface for on-demand queries
    """
    
    def __init__(
        self,
        pub_endpoint: str = "tcp://*:5557",
        rep_endpoint: str = "tcp://*:5558",
        logger=None
    ):
        """Initialize the publisher."""
        self.logger = logger or get_logger("token_tracking_publisher")
        self.pub_endpoint = pub_endpoint
        self.rep_endpoint = rep_endpoint
        
        # Reference to cache for data access
        self.cache = None
        
        # ZMQ context and sockets
        self._context: Optional[zmq.asyncio.Context] = None
        self._pub_socket: Optional[zmq.asyncio.Socket] = None
        self._rep_socket: Optional[zmq.asyncio.Socket] = None
        
        # State
        self._is_running = False
        self._req_rep_task = None
        self._last_published_block = 0
    
    def set_cache(self, cache):
        """Set reference to token tracking cache for data access."""
        self.cache = cache
    
    async def start(self):
        """Start the publisher."""
        if self._is_running:
            return
        
        self.logger.info("Starting token tracking publisher")
        self._is_running = True
        
        try:
            # Initialize ZMQ
            self._context = zmq.asyncio.Context()
            
            # Set up PUB socket
            self._pub_socket = self._context.socket(zmq.PUB)
            self._pub_socket.bind(self.pub_endpoint)
            self.logger.info(f"PUB socket bound to {self.pub_endpoint}")
            
            # Set up REP socket
            self._rep_socket = self._context.socket(zmq.REP)
            self._rep_socket.bind(self.rep_endpoint)
            self.logger.info(f"REP socket bound to {self.rep_endpoint}")
            
            # Start REQ/REP handler
            self._req_rep_task = asyncio.create_task(self._handle_requests())
            
        except Exception as e:
            self.logger.error(f"Error starting publisher: {e}")
            await self.stop()
            raise
    
    async def stop(self):
        """Stop the publisher."""
        if not self._is_running:
            return
        
        self.logger.info("Stopping token tracking publisher")
        self._is_running = False
        
        # Cancel request handler
        if self._req_rep_task and not self._req_rep_task.done():
            self._req_rep_task.cancel()
            try:
                await self._req_rep_task
            except asyncio.CancelledError:
                self.logger.debug("Request handler task cancelled during shutdown")
        
        # Close sockets
        if self._pub_socket:
            self._pub_socket.close()
        if self._rep_socket:
            self._rep_socket.close()
        
        # Terminate context
        if self._context:
            self._context.term()
        
        self.logger.info("Token tracking publisher stopped")
    
    async def publish_all(self):
        """
        Publish all tokens from cache.
        Used at startup or for full resync.
        """
        if not self._is_running or not self._pub_socket or not self.cache:
            return
        
        try:
            # Get all tokens from cache
            all_tokens = await self.cache.get_all_tokens_for_publishing()
            
            if all_tokens:
                message = {
                    'type': 'full_update',
                    'timestamp': time.time(),
                    'token_count': len(all_tokens),
                    'data': all_tokens
                }
                
                json_data = json.dumps(message)
                await self._pub_socket.send_string(json_data)
                
                self.logger.info(f"Published full update with {len(all_tokens)} tokens")
            
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
            # Get only the tokens that were updated in this block
            updated_tokens = await self.cache.get_updated_tokens_for_publishing()
            
            if updated_tokens:
                message = {
                    'type': 'block_update',
                    'timestamp': time.time(),
                    'block_number': block_number,
                    'token_count': len(updated_tokens),
                    'data': updated_tokens
                }
                
                json_data = json.dumps(message)
                await self._pub_socket.send_string(json_data)
                
                self._last_published_block = block_number
                self.logger.debug(f"Published block {block_number} update with {len(updated_tokens)} updated tokens")
            else:
                self.logger.debug(f"No tokens updated in block {block_number}, skipping publish")
            
        except Exception as e:
            self.logger.error(f"Error publishing block update: {e}")
    
    async def _handle_requests(self):
        """Handle REQ/REP requests for token data."""
        self.logger.info("Starting request handler")
        
        while self._is_running:
            try:
                # Wait for request
                request_json = await self._rep_socket.recv_string()
                request = json.loads(request_json)
                
                # Process request
                response = await self._process_request(request)
                
                # Send response
                await self._rep_socket.send_string(json.dumps(response))
                
            except asyncio.CancelledError:
                break
            except Exception as e:
                self.logger.error(f"Error handling request: {e}")
                
                # Send error response
                try:
                    await self._rep_socket.send_string(json.dumps({
                        'status': 'error',
                        'error': str(e)
                    }))
                except Exception as send_error:
                    self.logger.error(f"Failed to send error response to client: {send_error}")
                
                await asyncio.sleep(0.1)
        
        self.logger.info("Request handler stopped")
    
    async def _process_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Process a client request."""
        if not self.cache:
            return {'status': 'error', 'error': 'Cache not initialized'}
        
        # Check if 'type' field exists
        if 'type' not in request:
            return {'status': 'error', 'error': 'Missing required field: type'}
        
        request_type = request['type']
        
        if request_type == 'get_all_tokens':
            # Get all tokens
            all_tokens = await self.cache.get_all_tokens_for_publishing()
            
            return {
                'status': 'success',
                'count': len(all_tokens),
                'data': all_tokens
            }
        
        elif request_type == 'get_stats':
            # Get statistics
            stats = self.cache.get_stats()
            return {
                'status': 'success',
                'stats': stats
            }
        
        else:
            return {
                'status': 'error',
                'error': f'Unknown request type: {request_type}'
            }