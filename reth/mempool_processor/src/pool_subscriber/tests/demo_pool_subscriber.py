#!/usr/bin/env python3
"""
Demo script to test pool_subscriber functionality
This simulates the Python publisher side for testing
"""

import zmq
import json
import time
import threading

def run_publisher():
    """Simulate Python pool publisher"""
    context = zmq.Context()
    publisher = context.socket(zmq.PUB)
    publisher.bind("tcp://127.0.0.1:25557")
    
    print("📡 Python mock publisher started on port 25557")
    time.sleep(0.5)  # Give time for binding
    
    # Send some test pool updates
    updates = [
        {
            "type": "pool_updates",
            "timestamp": time.time(),
            "data": {
                "0xPool1111111111111111111111111111111111": {
                    "eth_reserve": 25.5,
                    "token_address": "0xToken1",
                    "block_number": 1000000,
                    "update_time": time.time()
                },
                "0xPool2222222222222222222222222222222222": {
                    "eth_reserve": 150.75,
                    "token_address": "0xToken2", 
                    "block_number": 1000001,
                    "update_time": time.time()
                }
            }
        },
        {
            "type": "pool_updates",
            "timestamp": time.time() + 1,
            "data": {
                "0xPool3333333333333333333333333333333333": {
                    "eth_reserve": 0.05,  # Below threshold
                    "token_address": "0xToken3",
                    "block_number": 1000002,
                    "update_time": time.time() + 1
                }
            }
        }
    ]
    
    for i, update in enumerate(updates):
        time.sleep(0.5)
        msg = json.dumps(update)
        publisher.send_string(msg)
        print(f"✅ Sent update {i+1}: {len(update['data'])} pools")
        for pool, data in update['data'].items():
            print(f"   - {pool[:10]}...: {data['eth_reserve']} ETH")
    
    # Keep alive for a bit
    time.sleep(2)
    print("📡 Publisher shutting down")

def run_rep_server():
    """Simulate Python REP server for initial data requests"""
    context = zmq.Context()
    rep_server = context.socket(zmq.REP)
    rep_server.bind("tcp://127.0.0.1:25558")
    rep_server.setsockopt(zmq.RCVTIMEO, 5000)  # 5 second timeout
    
    print("🔄 Python mock REP server started on port 25558")
    
    try:
        request = rep_server.recv_string()
        print(f"📥 Received request: {request}")
        
        if "get_all_pools" in request:
            response = {
                "status": "success",
                "count": 2,
                "data": {
                    "0xInitPool111111111111111111111111111111": {
                        "eth_reserve": 500.0,
                        "token_address": "0xInitToken1",
                        "block_number": 999999,
                        "update_time": time.time()
                    },
                    "0xInitPool222222222222222222222222222222": {
                        "eth_reserve": 75.25,
                        "token_address": "0xInitToken2",
                        "block_number": 999998,
                        "update_time": time.time()
                    }
                }
            }
            
            rep_server.send_string(json.dumps(response))
            print("✅ Sent initial pool data (2 pools)")
    except zmq.error.Again:
        print("⏱️  REP server timeout - no requests received")
    
    print("🔄 REP server shutting down")

def main():
    print("\n🚀 Pool Subscriber Test Demo")
    print("=" * 50)
    print("This script simulates the Python side of pool publishing")
    print("Run the Rust subscriber to receive these updates\n")
    
    # Run REP server in thread
    rep_thread = threading.Thread(target=run_rep_server)
    rep_thread.start()
    
    # Run publisher
    run_publisher()
    
    # Wait for REP thread
    rep_thread.join()
    
    print("\n✅ Demo completed!")

if __name__ == "__main__":
    main()