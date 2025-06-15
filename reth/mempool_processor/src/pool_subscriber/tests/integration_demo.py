#!/usr/bin/env python3
"""
Integration demo showing Python publisher → Rust subscriber communication.

This demonstrates the complete flow:
1. Python publishes pool updates via ZeroMQ
2. Rust subscriber receives and caches the data
3. Pool limit management happens on Python side
"""

import asyncio
import json
import time
import zmq
import zmq.asyncio
from typing import Dict, Any


async def simulate_pool_updates():
    """Simulate a Python publisher sending pool updates."""
    
    context = zmq.asyncio.Context()
    pub_socket = context.socket(zmq.PUB)
    rep_socket = context.socket(zmq.REP)
    
    try:
        # Bind sockets
        pub_socket.bind("tcp://*:15557")  # Use different ports to avoid conflicts
        rep_socket.bind("tcp://*:15558")
        
        print("Demo Publisher started on ports 15557/15558")
        print("To test with Rust subscriber:")
        print('  cargo run --example pool_subscriber_live -- --endpoint "tcp://localhost:15557"')
        print()
        
        # Simulate pool data with varying liquidity
        pools = {}
        
        # Add initial high-liquidity pools
        for i in range(5):
            addr = f"0x{i:040x}"
            pools[addr] = {
                "eth_reserve": 100.0 + i * 50,
                "token_address": f"0x{(i+100):040x}",
                "block_number": 20000000,
                "update_time": time.time()
            }
        
        # Publish initial data
        message = {
            'type': 'pool_updates',
            'timestamp': time.time(),
            'data': pools
        }
        await pub_socket.send_string(json.dumps(message))
        print(f"Published {len(pools)} initial high-liquidity pools")
        
        # Handle REQ/REP in background
        async def handle_requests():
            while True:
                try:
                    request = await asyncio.wait_for(rep_socket.recv_string(), timeout=0.5)
                    req_data = json.loads(request)
                    
                    if req_data.get('type') == 'get_all_pools':
                        response = {
                            'status': 'success',
                            'count': len(pools),
                            'data': pools
                        }
                    elif req_data.get('type') == 'get_pool_stats':
                        eth_reserves = [p['eth_reserve'] for p in pools.values()]
                        response = {
                            'status': 'success',
                            'stats': {
                                'pool_count': len(pools),
                                'max_pools': 2000,
                                'total_eth_locked': sum(eth_reserves),
                                'average_eth_per_pool': sum(eth_reserves) / len(eth_reserves) if eth_reserves else 0,
                                'pools_below_threshold': sum(1 for eth in eth_reserves if eth < 0.01)
                            }
                        }
                    else:
                        response = {'status': 'error', 'error': 'Unknown request'}
                    
                    await rep_socket.send_string(json.dumps(response))
                except asyncio.TimeoutError:
                    continue
        
        # Start request handler
        req_task = asyncio.create_task(handle_requests())
        
        # Simulate ongoing updates
        for update_num in range(10):
            await asyncio.sleep(2)
            
            # Add some low-liquidity pools
            new_pools = {}
            for i in range(3):
                idx = 100 + update_num * 10 + i
                addr = f"0x{idx:040x}"
                # Mix of high and low liquidity
                eth_reserve = 0.001 if i == 0 else (5.0 + i)
                
                new_pools[addr] = {
                    "eth_reserve": eth_reserve,
                    "token_address": f"0x{(idx+1000):040x}",
                    "block_number": 20000001 + update_num,
                    "update_time": time.time()
                }
                pools[addr] = new_pools[addr]
            
            # Simulate pool limit management (keep only top 10 by ETH)
            if len(pools) > 10:
                sorted_pools = sorted(pools.items(), key=lambda x: x[1]['eth_reserve'], reverse=True)
                pools = dict(sorted_pools[:10])
                print(f"Cleaned up to top 10 pools (simulating 2K limit)")
            
            # Publish update
            message = {
                'type': 'pool_updates',
                'timestamp': time.time(),
                'data': new_pools
            }
            await pub_socket.send_string(json.dumps(message))
            
            print(f"Update {update_num + 1}: Published {len(new_pools)} pools, total cached: {len(pools)}")
            
        print("\nDemo complete!")
        
    finally:
        req_task.cancel()
        pub_socket.close()
        rep_socket.close()
        context.term()


async def main():
    """Run the integration demo."""
    print("Pool Subscriber Integration Demo")
    print("================================")
    print("This demonstrates Python → Rust communication")
    print()
    
    try:
        await simulate_pool_updates()
    except KeyboardInterrupt:
        print("\nDemo stopped")


if __name__ == "__main__":
    asyncio.run(main())