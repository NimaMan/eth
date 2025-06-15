#!/usr/bin/env python3
"""
Test Python publisher for pool_subscriber integration testing.

This script simulates the Python pool level publisher to test the Rust subscriber.
Run this alongside the Rust integration test to verify end-to-end functionality.

Usage:
    python python_publisher_test.py [--ports 5557,5558]
"""

import asyncio
import json
import time
import zmq
import zmq.asyncio
import sys
from typing import Dict, Any


class TestPoolPublisher:
    """Test publisher that simulates the real PoolLevelPublisher."""
    
    def __init__(self, pub_port=5557, rep_port=5558):
        self.pub_port = pub_port
        self.rep_port = rep_port
        self.context = None
        self.pub_socket = None
        self.rep_socket = None
        self.pool_data = {}
        self.is_running = False
        
    async def start(self):
        """Start the test publisher."""
        print(f"Starting test pool publisher on ports {self.pub_port}/{self.rep_port}")
        
        self.context = zmq.asyncio.Context()
        
        # Create PUB socket
        self.pub_socket = self.context.socket(zmq.PUB)
        self.pub_socket.bind(f"tcp://*:{self.pub_port}")
        print(f"✓ PUB socket bound to port {self.pub_port}")
        
        # Create REP socket
        self.rep_socket = self.context.socket(zmq.REP)
        self.rep_socket.bind(f"tcp://*:{self.rep_port}")
        print(f"✓ REP socket bound to port {self.rep_port}")
        
        self.is_running = True
        
        # Start tasks
        await asyncio.gather(
            self._handle_requests(),
            self._publish_updates()
        )
        
    async def _handle_requests(self):
        """Handle REQ/REP requests."""
        print("Starting request handler...")
        
        while self.is_running:
            try:
                # Wait for request with timeout
                request_data = await asyncio.wait_for(
                    self.rep_socket.recv_string(),
                    timeout=1.0
                )
                
                request = json.loads(request_data)
                print(f"Received request: {request.get('type', 'unknown')}")
                
                # Handle request types
                if request.get('type') == 'get_all_pools':
                    response = {
                        'status': 'success',
                        'count': len(self.pool_data),
                        'data': self.pool_data
                    }
                elif request.get('type') == 'get_pool_stats':
                    eth_reserves = [p.get('eth_reserve', 0) for p in self.pool_data.values()]
                    response = {
                        'status': 'success',
                        'stats': {
                            'pool_count': len(self.pool_data),
                            'max_pools': 2000,
                            'total_eth_locked': sum(eth_reserves) if eth_reserves else 0,
                            'average_eth_per_pool': sum(eth_reserves) / len(eth_reserves) if eth_reserves else 0,
                            'min_eth_reserve': min(eth_reserves) if eth_reserves else 0,
                            'max_eth_reserve': max(eth_reserves) if eth_reserves else 0,
                            'pools_below_threshold': sum(1 for eth in eth_reserves if eth < 0.01),
                            'eth_threshold': 0.01,
                            'capacity_used_percent': len(self.pool_data) / 2000 * 100
                        }
                    }
                else:
                    response = {
                        'status': 'error',
                        'error': f'Unknown request type: {request.get("type")}'
                    }
                
                await self.rep_socket.send_string(json.dumps(response))
                print(f"Sent response with {len(response.get('data', {}))} pools")
                
            except asyncio.TimeoutError:
                continue
            except Exception as e:
                print(f"Error handling request: {e}")
                
    async def _publish_updates(self):
        """Publish pool updates periodically."""
        print("Starting update publisher...")
        
        # Wait a bit for subscribers to connect
        await asyncio.sleep(0.5)
        
        # Initial pool data
        initial_pools = {
            "0x1234567890123456789012345678901234567890": {
                "eth_reserve": 100.5,
                "token_address": "0xAABBCCDDEEFF112233445566778899AABBCCDDEE",
                "block_number": 20000000,
                "update_time": time.time()
            },
            "0x2345678901234567890123456789012345678901": {
                "eth_reserve": 50.25,
                "token_address": "0xBBCCDDEEFF112233445566778899AABBCCDDEEFF",
                "block_number": 20000000,
                "update_time": time.time()
            },
            "0x3456789012345678901234567890123456789012": {
                "eth_reserve": 25.75,
                "token_address": "0xCCDDEEFF112233445566778899AABBCCDDEEFF11",
                "block_number": 20000000,
                "update_time": time.time()
            }
        }
        
        # Store in our data
        self.pool_data.update(initial_pools)
        
        # Publish initial update
        message = {
            'type': 'pool_updates',
            'timestamp': time.time(),
            'data': initial_pools
        }
        
        await self.pub_socket.send_string(json.dumps(message))
        print(f"Published initial update with {len(initial_pools)} pools")
        
        # Simulate ongoing updates
        update_count = 0
        while self.is_running and update_count < 10:
            await asyncio.sleep(2)  # Update every 2 seconds
            
            # Create an update (modify existing pool and add new one)
            updated_pools = {}
            
            # Update existing pool
            if update_count % 2 == 0:
                pool_addr = "0x1234567890123456789012345678901234567890"
                new_reserve = 100.5 + (update_count * 5)
                updated_pools[pool_addr] = {
                    "eth_reserve": new_reserve,
                    "token_address": "0xAABBCCDDEEFF112233445566778899AABBCCDDEE",
                    "block_number": 20000001 + update_count,
                    "update_time": time.time()
                }
                self.pool_data[pool_addr] = updated_pools[pool_addr]
            
            # Add new pool
            new_addr = f"0x{update_count:040x}"
            updated_pools[new_addr] = {
                "eth_reserve": 10.0 + update_count,
                "token_address": f"0x{(update_count + 100):040x}",
                "block_number": 20000001 + update_count,
                "update_time": time.time()
            }
            self.pool_data[new_addr] = updated_pools[new_addr]
            
            # Publish update
            message = {
                'type': 'pool_updates',
                'timestamp': time.time(),
                'data': updated_pools
            }
            
            await self.pub_socket.send_string(json.dumps(message))
            print(f"Published update {update_count + 1} with {len(updated_pools)} pool changes")
            
            update_count += 1
            
        print("Update publisher finished")
        
    async def stop(self):
        """Stop the publisher."""
        self.is_running = False
        
        if self.pub_socket:
            self.pub_socket.close()
        if self.rep_socket:
            self.rep_socket.close()
        if self.context:
            self.context.term()
            
        print("Test publisher stopped")


async def main():
    """Run the test publisher."""
    # Parse command line args
    pub_port = 5557
    rep_port = 5558
    
    if len(sys.argv) > 1 and sys.argv[1] == '--ports':
        ports = sys.argv[2].split(',')
        pub_port = int(ports[0])
        rep_port = int(ports[1])
    
    publisher = TestPoolPublisher(pub_port, rep_port)
    
    try:
        await publisher.start()
    except KeyboardInterrupt:
        print("\nShutting down...")
    finally:
        await publisher.stop()


if __name__ == "__main__":
    print("Pool Subscriber Test Publisher")
    print("==============================")
    asyncio.run(main())