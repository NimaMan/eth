"""
PoolLevelPublisher: Publish pool level updates to the Rust mempool processor.

Objective:
---------
1. Share pool ETH reserve levels with the Rust mempool processor
2. Support both push-based (PUB/SUB) and pull-based (REQ/REP) communication
3. Provide efficient serialization of pool data
4. Handle errors and reconnections gracefully

This component is responsible for establishing the data bridge between the
Python token processing pipeline and the Rust mempool processor for scam
detection.
"""

import asyncio
import zmq
import zmq.asyncio
import json
import time
from typing import Dict, Any, Optional, Set, List
import threading
import logging
from eth_portfolio_manager.utils.logger import get_logger


class PoolLevelPublisher:
    """
    Publishes pool level updates to the Rust mempool processor.
    
    This class provides both:
    1. A PUB/SUB interface for pushing updates as they occur
    2. A REQ/REP interface for responding to on-demand requests
    """
    
    def __init__(
        self,
        pub_endpoint: str = "tcp://*:5557",
        rep_endpoint: str = "tcp://*:5558",
        logger=None,
    ):
        """
        Initialize the pool level publisher.
        
        Args:
            pub_endpoint: ZMQ PUB socket endpoint for pushing updates
            rep_endpoint: ZMQ REP socket endpoint for responding to requests
            logger: Optional logger instance
        """
        self.logger = logger or get_logger("pool_level_publisher")
        self.pub_endpoint = pub_endpoint
        self.rep_endpoint = rep_endpoint
        
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
            
        self.logger.info(f"Starting pool level publisher")
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
            
        self.logger.info("Stopping pool level publisher")
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
            
        self.logger.info("Pool level publisher stopped")
    
    async def update_pool_levels(self, updated_pools: Dict[str, Dict[str, Any]]):
        """
        Update pool levels and publish changes.
        
        Args:
            updated_pools: Dict of pool address to pool data
        """
        if not updated_pools or not self._is_running:
            return
            
        try:
            # Update internal state
            async with self._lock:
                self._pool_data.update(updated_pools)
                
            # Publish updates
            await self._publish_updates(updated_pools)
            
        except Exception as e:
            self.logger.error(f"Error updating pool levels: {e}")
    
    async def _publish_updates(self, updated_pools: Dict[str, Dict[str, Any]]):
        """
        Publish pool updates to subscribers.
        
        Args:
            updated_pools: Dict of pool address to pool data
        """
        if not self._pub_socket or not updated_pools:
            return
            
        try:
            # Create a message with all updates
            message = {
                'type': 'pool_updates',
                'timestamp': time.time(),
                'data': updated_pools
            }
            
            # Serialize and send
            json_data = json.dumps(message)
            await self._pub_socket.send_string(json_data)
            
            self.logger.debug(f"Published {len(updated_pools)} pool updates")
            
        except Exception as e:
            self.logger.error(f"Error publishing pool updates: {e}")
    
    async def _handle_requests(self):
        """Handle REQ/REP requests for pool data."""
        self.logger.info("Starting pool data request handler")
        
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
                self.logger.error(f"Error handling pool data request: {e}")
                
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
                
        self.logger.info("Pool data request handler stopped")
    
    async def _process_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process a client request for pool data.
        
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
            
        else:
            # Unknown request type
            return {
                'status': 'error',
                'error': f'Unknown request type: {request_type}'
            } 