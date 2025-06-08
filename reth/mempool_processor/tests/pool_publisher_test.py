#!/usr/bin/env python3
"""
Test pool publisher for live pool detection testing.
This script publishes pool updates via ZMQ for testing the Rust pool detection system.
"""

import zmq
import json
import time
import random
from datetime import datetime

# Popular token pools on Ethereum mainnet (these are real pool addresses)
TEST_POOLS = {
    # USDC/WETH pools
    "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640": {
        "token_address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",  # USDC
        "token_symbol": "USDC",
        "eth_reserve": 125.5
    },
    "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8": {
        "token_address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",  # USDC
        "token_symbol": "USDC",
        "eth_reserve": 89.3
    },
    
    # USDT pools
    "0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36": {
        "token_address": "0xdAC17F958D2ee523a2206206994597C13D831ec7",  # USDT
        "token_symbol": "USDT",
        "eth_reserve": 67.8
    },
    
    # DAI pools
    "0xC2e9F25Be6257c210d7Adf0D4Cd6E3E881ba25f8": {
        "token_address": "0x6B175474E89094C44Da98b954EedeAC495271d0F",  # DAI
        "token_symbol": "DAI",
        "eth_reserve": 45.2
    },
    
    # SHIB pool (high activity)
    "0x8dB1b906d47dFc1D84A87fc49bd0522e285b98b9": {
        "token_address": "0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE",  # SHIB
        "token_symbol": "SHIB",
        "eth_reserve": 234.7
    },
    
    # PEPE pool (meme token with activity)
    "0x11950d141EcB863F01007AdD7D1A342041227b58": {
        "token_address": "0x6982508145454Ce325dDbE47a25d4ec3d2311933",  # PEPE
        "token_symbol": "PEPE",
        "eth_reserve": 156.9
    },
    
    # LINK pool
    "0xa6Cc3C2531FdaA6Ae1A3CA84c2855806728693e8": {
        "token_address": "0x514910771AF9Ca656af840dff83E8264EcF986CA",  # LINK
        "token_symbol": "LINK",
        "eth_reserve": 78.4
    },
    
    # MATIC pool
    "0x290A6a7460B308ee3F19023D2D00dE604bcf5B42": {
        "token_address": "0x7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0",  # MATIC
        "token_symbol": "MATIC",
        "eth_reserve": 92.1
    }
}

def publish_pool_updates(duration_seconds=70):
    """Publish pool updates for testing."""
    
    # Setup ZMQ publisher
    context = zmq.Context()
    socket = context.socket(zmq.PUB)
    socket.bind("tcp://*:5557")
    
    print(f"🚀 Pool publisher started on tcp://localhost:5557")
    print(f"📊 Publishing {len(TEST_POOLS)} test pools")
    print(f"⏱️  Will run for {duration_seconds} seconds")
    print("-" * 60)
    
    start_time = time.time()
    update_count = 0
    
    try:
        while time.time() - start_time < duration_seconds:
            # Create pool update message
            pool_data = {}
            
            for pool_address, pool_info in TEST_POOLS.items():
                # Add some randomness to ETH reserves to simulate market activity
                base_reserve = pool_info["eth_reserve"]
                variation = random.uniform(-0.05, 0.05)  # ±5% variation
                current_reserve = base_reserve * (1 + variation)
                
                pool_data[pool_address] = {
                    "eth_reserve": current_reserve,
                    "token_address": pool_info["token_address"],
                    "block_number": 17900000 + update_count,  # Simulate block progression
                    "update_time": time.time()
                }
            
            # Create message in expected format
            message = {
                "type": "pool_updates",
                "timestamp": time.time(),
                "data": pool_data
            }
            
            # Publish message
            socket.send_json(message)
            update_count += 1
            
            # Log every 10 updates
            if update_count % 10 == 0:
                elapsed = time.time() - start_time
                print(f"📤 Update #{update_count} sent ({elapsed:.1f}s elapsed)")
                print(f"   Pools: {len(pool_data)}, "
                      f"Sample ETH: {list(pool_data.values())[0]['eth_reserve']:.4f}")
            
            # Send updates every 2 seconds
            time.sleep(2)
            
    except KeyboardInterrupt:
        print("\n⚠️  Interrupted by user")
    finally:
        print(f"\n📊 Published {update_count} updates in {time.time() - start_time:.1f} seconds")
        socket.close()
        context.term()

def test_single_message():
    """Send a single test message for debugging."""
    context = zmq.Context()
    socket = context.socket(zmq.PUB)
    socket.bind("tcp://*:5557")
    
    time.sleep(1)  # Give socket time to bind
    
    # Send one test message
    message = {
        "type": "pool_updates",
        "timestamp": time.time(),
        "data": {
            "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640": {
                "eth_reserve": 125.5,
                "token_address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
                "block_number": 17900000,
                "update_time": time.time()
            }
        }
    }
    
    socket.send_json(message)
    print("✅ Test message sent!")
    print(json.dumps(message, indent=2))
    
    socket.close()
    context.term()

if __name__ == "__main__":
    import sys
    
    if len(sys.argv) > 1 and sys.argv[1] == "--test":
        test_single_message()
    else:
        # Run for 70 seconds (10 seconds longer than the Rust test)
        publish_pool_updates(duration_seconds=70)