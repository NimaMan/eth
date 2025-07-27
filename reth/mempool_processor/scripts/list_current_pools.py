#!/usr/bin/env python3
"""
Script to query all current pools from the Rust pool subscriber
and identify pools with 0 ETH reserves.
"""

import zmq
import json
import time
from datetime import datetime

def get_all_pools():
    """Connect to Rust pool subscriber and get all current pools."""
    context = zmq.Context()
    requester = context.socket(zmq.REQ)
    
    try:
        # Connect to the Rust REQ/REP endpoint
        requester.connect("tcp://127.0.0.1:5557")
        requester.setsockopt(zmq.RCVTIMEO, 5000)  # 5 second timeout
        
        print(f"[{datetime.now()}] Requesting all pools from Rust pool subscriber...")
        
        # Send request for all pools
        request = json.dumps({
            "type": "get_all_pools"
        })
        requester.send_string(request)
        
        # Wait for response
        response = requester.recv_string()
        data = json.loads(response)
        
        if data.get("status") == "success":
            pools = data.get("pools", {})
            print(f"\nReceived {len(pools)} pools from Rust")
            
            # Find pools with 0 ETH
            zero_eth_pools = []
            low_eth_pools = []
            
            for pool_addr, pool_data in pools.items():
                eth_reserve = pool_data.get("eth_reserve", 0)
                
                if eth_reserve == 0:
                    zero_eth_pools.append((pool_addr, pool_data))
                elif eth_reserve < 0.01:
                    low_eth_pools.append((pool_addr, pool_data))
            
            # Print statistics
            print(f"\nPool Statistics:")
            print(f"  Total pools tracked: {len(pools)}")
            print(f"  Pools with 0 ETH: {len(zero_eth_pools)}")
            print(f"  Pools with < 0.01 ETH: {len(low_eth_pools)}")
            
            # Show examples of 0 ETH pools
            if zero_eth_pools:
                print(f"\nExamples of pools with 0 ETH reserves:")
                for i, (pool_addr, pool_data) in enumerate(zero_eth_pools[:5]):
                    print(f"\n  {i+1}. Pool: {pool_addr}")
                    print(f"     Token: {pool_data.get('token_address', 'Unknown')}")
                    print(f"     Symbol: {pool_data.get('token_symbol', 'Unknown')}")
                    print(f"     ETH Reserve: {pool_data.get('eth_reserve', 0):.6f}")
                    print(f"     Token Reserve: {pool_data.get('token_reserve', 0):.2f}")
                    print(f"     Pool Type: {pool_data.get('pool_type', 'Unknown')}")
                    print(f"     Last Update: Block {pool_data.get('block_number', 0)}")
            
            # Show examples of low ETH pools
            if low_eth_pools:
                print(f"\n\nExamples of pools with < 0.01 ETH reserves:")
                for i, (pool_addr, pool_data) in enumerate(low_eth_pools[:5]):
                    print(f"\n  {i+1}. Pool: {pool_addr}")
                    print(f"     Token: {pool_data.get('token_address', 'Unknown')}")
                    print(f"     Symbol: {pool_data.get('token_symbol', 'Unknown')}")
                    print(f"     ETH Reserve: {pool_data.get('eth_reserve', 0):.6f}")
                    print(f"     Token Reserve: {pool_data.get('token_reserve', 0):.2f}")
                    print(f"     Pool Type: {pool_data.get('pool_type', 'Unknown')}")
                    print(f"     Last Update: Block {pool_data.get('block_number', 0)}")
            
            # Save full list to file for analysis
            output_file = f"pool_snapshot_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
            with open(output_file, 'w') as f:
                json.dump({
                    "timestamp": datetime.now().isoformat(),
                    "total_pools": len(pools),
                    "zero_eth_pools": len(zero_eth_pools),
                    "low_eth_pools": len(low_eth_pools),
                    "pools": pools
                }, f, indent=2)
            
            print(f"\n\nFull pool data saved to: {output_file}")
            
            return pools
            
        else:
            print(f"Error response: {data}")
            return None
            
    except zmq.Again:
        print("Timeout waiting for response. Is the Rust pool subscriber running?")
        return None
    except Exception as e:
        print(f"Error querying pools: {e}")
        return None
    finally:
        requester.close()
        context.term()

def main():
    """Main function."""
    print("=" * 80)
    print("Pool Snapshot Tool - Query all pools from Rust pool subscriber")
    print("=" * 80)
    
    # Check if pool subscriber is running
    pools = get_all_pools()
    
    if pools:
        print("\nAnalysis complete!")
    else:
        print("\nFailed to get pool data. Make sure:")
        print("  1. The Rust pool subscriber is running")
        print("  2. The Python pool publisher is sending data")
        print("  3. The REQ/REP endpoint is available on port 5557")

if __name__ == "__main__":
    main()