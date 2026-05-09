#!/usr/bin/env python3
"""
Utility to check if Reth is running and provide guidance
"""

import subprocess
import sys

def check_reth_running():
    """Check if Reth is running"""
    try:
        result = subprocess.run(['pgrep', '-f', 'reth node'], capture_output=True, text=True)
        return result.returncode == 0
    except:
        return False

def check_rpc_available():
    """Check if Reth RPC is available"""
    try:
        import requests
        response = requests.post('http://localhost:8545', 
                                json={'jsonrpc': '2.0', 'method': 'eth_blockNumber', 'params': [], 'id': 1},
                                timeout=1)
        return response.status_code == 200
    except:
        return False

def main():
    if check_reth_running():
        print("❌ Reth is currently running and has the database locked.")
        print("\nYou have two options:")
        print("1. Use Reth's RPC interface at http://localhost:8545")
        print("2. Stop Reth temporarily (NOT RECOMMENDED)")
        print("\nFor PyReth examples, Reth must be stopped since PyReth needs direct database access.")
        print("However, DO NOT stop Reth if it's syncing or being used by other services!")
        
        if check_rpc_available():
            print("\n✅ Reth RPC is available at http://localhost:8545")
            print("Consider using RPC-based tools instead of PyReth for now.")
        
        return False
    else:
        print("✅ Reth is not running. PyReth can access the database directly.")
        return True

if __name__ == "__main__":
    can_run = main()
    sys.exit(0 if can_run else 1)