"""
TokenInfoPublisher: Publish token and pool information updates to the Rust mempool processor.

Objective:
---------
1. Share pool ETH reserve levels with the Rust mempool processor
2. Support both push-based (PUB/SUB) and pull-based (REQ/REP) communication
3. Provide efficient serialization of pool data
4. Handle errors and reconnections gracefully

This component is responsible for establishing the data bridge between the
Python token processing pipeline and the Rust mempool processor for scam
detection and trading decisions.
"""

import asyncio
import zmq
import zmq.asyncio
import json
import time
from typing import Dict, Any, Optional, Set, List
from eth_portfolio_manager.utils.logger import get_logger


class TokenInfoPublisher:
    """
    Publishes token and pool information updates to the Rust mempool processor.
    
    This class provides both:
    1. A PUB/SUB interface for pushing updates as they occur
    2. A REQ/REP interface for responding to on-demand requests
    """
    
    def __init__(
        self,
        pub_endpoint: str = "tcp://*:5557",
        rep_endpoint: str = "tcp://*:5558",
        logger=None,
        max_pools: int = 2000,
        min_eth_threshold: float = 0.05,
    ):
        """
        Initialize the token info publisher.
        
        Args:
            pub_endpoint: ZMQ PUB socket endpoint for pushing updates
            rep_endpoint: ZMQ REP socket endpoint for responding to requests
            logger: Optional logger instance
            max_pools: Maximum number of pools to maintain (default 2000)
            min_eth_threshold: Minimum ETH reserve to keep pool when at capacity (default 0.05)
        """
        self.logger = logger or get_logger("token_info_publisher")
        self.pub_endpoint = pub_endpoint
        self.rep_endpoint = rep_endpoint
        self.max_pools = max_pools
        self.min_eth_threshold = min_eth_threshold
        
        # ZMQ context and sockets
        self._context: Optional[zmq.asyncio.Context] = None
        self._pub_socket: Optional[zmq.asyncio.Socket] = None
        self._rep_socket: Optional[zmq.asyncio.Socket] = None
        
        # State
        self._pool_data: Dict[str, Dict[str, Any]] = {}
        self._is_running = False
        self._lock = asyncio.Lock()
        
        # Tasks
        self._req_rep_task = None
    
    async def start(self):
        """Start the publisher."""
        if self._is_running:
            return
            
        self.logger.info(f"Starting token info publisher")
        self._is_running = True
        
        # Initialize ZMQ
        try:
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
            self.logger.error(f"Error starting pool level publisher: {e}")
            await self.stop()
            raise
    
    async def stop(self):
        """Stop the publisher and clean up resources."""
        if not self._is_running:
            return
            
        self.logger.info("Stopping token info publisher")
        self._is_running = False
        
        # Cancel tasks
        if self._req_rep_task and not self._req_rep_task.done():
            self._req_rep_task.cancel()
            try:
                await self._req_rep_task
            except asyncio.CancelledError:
                pass
        
        # Close sockets
        if self._pub_socket:
            self._pub_socket.close()
            self._pub_socket = None
            
        if self._rep_socket:
            self._rep_socket.close()
            self._rep_socket = None
            
        # Terminate context
        if self._context:
            self._context.term()
            self._context = None
            
        self.logger.info("Token info publisher stopped")
    
    async def update_token_info(self, updated_pools: Dict[str, Dict[str, Any]]):
        """
        Update token information and publish changes.
        
        Args:
            updated_pools: Dict of pool address to pool data
        """
        if not updated_pools or not self._is_running:
            return
            
        try:
            # Update internal state
            async with self._lock:
                self._pool_data.update(updated_pools)
                
                # Check if we need to cleanup low-liquidity pools
                if len(self._pool_data) > self.max_pools:
                    await self._cleanup_low_liquidity_pools()
                
            # Publish updates
            await self._publish_updates(updated_pools)
            
        except Exception as e:
            self.logger.error(f"Error updating token info: {e}")
    
    async def _cleanup_low_liquidity_pools(self):
        """
        Remove pools with low or zero ETH reserves when we exceed max_pools limit.
        
        This method is called when the pool count exceeds max_pools.
        It sorts pools by ETH reserve and removes the ones with the lowest liquidity.
        """
        current_count = len(self._pool_data)
        pools_to_remove = current_count - self.max_pools
        
        if pools_to_remove <= 0:
            return
            
        self.logger.info(f"Pool count ({current_count}) exceeds limit ({self.max_pools}). "
                         f"Removing {pools_to_remove} low-liquidity pools.")
        
        # Sort pools by ETH reserve (ascending)
        sorted_pools = sorted(
            self._pool_data.items(),
            key=lambda x: x[1].get('eth_reserve', 0)
        )
        
        # Remove pools with lowest ETH reserves
        removed_count = 0
        removed_pools = []
        
        for pool_addr, pool_data in sorted_pools:
            eth_reserve = pool_data.get('eth_reserve', 0)
            
            # Always remove pools below threshold, or remove enough to get under limit
            if eth_reserve < self.min_eth_threshold or removed_count < pools_to_remove:
                del self._pool_data[pool_addr]
                removed_pools.append((pool_addr, eth_reserve))
                removed_count += 1
                
                # Stop if we've removed enough pools and remaining pools are above threshold
                if removed_count >= pools_to_remove and eth_reserve >= self.min_eth_threshold:
                    break
        
        # Log summary of removed pools
        if removed_pools:
            self.logger.info(f"Removed {len(removed_pools)} pools. "
                            f"Lowest removed: {removed_pools[0][1]:.4f} ETH, "
                            f"Highest removed: {removed_pools[-1][1]:.4f} ETH")
    
    async def _publish_updates(self, updated_pools: Dict[str, Dict[str, Any]]):
        """
        Publish token and pool updates to subscribers.
        
        Args:
            updated_pools: Dict of pool address to pool data
        """
        if not self._pub_socket or not updated_pools:
            return
            
        try:
            # Create a message with all updates
            # Ensure V4 pools have proper identification
            for pool_addr, pool_data in updated_pools.items():
                if pool_data.get('pool_type') == 'V4' and 'pool_id' in pool_data:
                    # Add pool_id at top level for easier access
                    pool_data['v4_pool_id'] = pool_data['pool_id']
                    pool_data['v4_pool_manager'] = pool_data.get('pool_manager', '0x000000000004444c5dc75cb358380d2e3de08a90')
            
            message = {
                'type': 'token_updates',
                'timestamp': time.time(),
                'data': updated_pools
            }
            
            # Serialize and send
            json_data = json.dumps(message)
            await self._pub_socket.send_string(json_data)
            
            self.logger.debug(f"Published {len(updated_pools)} token updates")
            
        except Exception as e:
            self.logger.error(f"Error publishing token updates: {e}")
    
    async def _handle_requests(self):
        """Handle REQ/REP requests for token and pool data."""
        self.logger.info("Starting token data request handler")
        
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
                self.logger.error(f"Error handling token data request: {e}")
                
                # Send error response if socket is still available
                if self._rep_socket:
                    try:
                        await self._rep_socket.send_string(json.dumps({
                            'status': 'error',
                            'error': str(e)
                        }))
                    except Exception:
                        pass
                        
                # Brief pause to avoid tight loop on persistent errors
                await asyncio.sleep(0.1)
                
        self.logger.info("Token data request handler stopped")
    
    async def _process_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process a client request for token and pool data.
        
        Args:
            request: Dict with request parameters
            
        Returns:
            Dict with response data
        """
        request_type = request.get('type', '')
        
        if request_type == 'get_pool':
            # Get data for a specific pool
            pool_address = request.get('pool_address')
            if not pool_address:
                return {'status': 'error', 'error': 'Missing pool_address'}
                
            async with self._lock:
                pool_data = self._pool_data.get(pool_address)
                
            if pool_data:
                return {
                    'status': 'success',
                    'data': pool_data
                }
            else:
                return {
                    'status': 'not_found',
                    'error': f'Pool {pool_address} not found'
                }
                
        elif request_type == 'get_all_pools':
            # Get data for all pools
            async with self._lock:
                pool_data = self._pool_data.copy()
                
            return {
                'status': 'success',
                'count': len(pool_data),
                'data': pool_data
            }
            
        elif request_type == 'get_pools_batch':
            # Get data for a batch of pools
            pool_addresses = request.get('pool_addresses', [])
            if not pool_addresses:
                return {'status': 'error', 'error': 'Missing pool_addresses'}
                
            async with self._lock:
                batch_data = {
                    addr: self._pool_data.get(addr) 
                    for addr in pool_addresses 
                    if addr in self._pool_data
                }
                
            return {
                'status': 'success',
                'count': len(batch_data),
                'data': batch_data
            }
            
        elif request_type == 'get_pool_stats':
            # Get statistics about the pool cache
            async with self._lock:
                pool_count = len(self._pool_data)
                if pool_count > 0:
                    eth_reserves = [p.get('eth_reserve', 0) for p in self._pool_data.values()]
                    total_eth = sum(eth_reserves)
                    avg_eth = total_eth / pool_count
                    min_eth = min(eth_reserves)
                    max_eth = max(eth_reserves)
                    below_threshold = sum(1 for eth in eth_reserves if eth < self.min_eth_threshold)
                else:
                    total_eth = avg_eth = min_eth = max_eth = 0
                    below_threshold = 0
                
            return {
                'status': 'success',
                'stats': {
                    'pool_count': pool_count,
                    'max_pools': self.max_pools,
                    'total_eth_locked': total_eth,
                    'average_eth_per_pool': avg_eth,
                    'min_eth_reserve': min_eth,
                    'max_eth_reserve': max_eth,
                    'pools_below_threshold': below_threshold,
                    'eth_threshold': self.min_eth_threshold,
                    'capacity_used_percent': (pool_count / self.max_pools * 100) if self.max_pools > 0 else 0
                }
            }
            
        else:
            # Unknown request type
            return {
                'status': 'error',
                'error': f'Unknown request type: {request_type}'
            } 