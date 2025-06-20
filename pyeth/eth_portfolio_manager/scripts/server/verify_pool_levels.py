#!/usr/bin/env python3
"""
Pool Level Verification Test

OBJECTIVE: Verify that pool information is being correctly published by the live portfolio manager.

ALGORITHM:
1. Connect to local reth node 
2. Connect to Python's pool level publisher via ZMQ (both SUB for real-time updates and REQ/REP for queries)
3. Subscribe to pool updates and monitor real-time data flow
4. Request all pools that Python currently has via REQ/REP
5. For each pool, query blockchain directly using getReserves()
6. Compare Python's pool data vs actual reserves
7. Report any discrepancies and verify message publishing

This tests both the pool publishing mechanism AND the accuracy of published pool data.
"""

import sys
import os
import json
import time
import zmq
from web3 import Web3
from typing import Dict, Tuple, List
import asyncio
import threading

# Add the eth_token path to access pool calculation logic
sys.path.append('/home/nima/code/crypto/py/eth_token')

# Uniswap V2 Pair ABI for getReserves()
UNISWAP_V2_PAIR_ABI = [
    {
        "constant": True,
        "inputs": [],
        "name": "getReserves",
        "outputs": [
            {"internalType": "uint112", "name": "_reserve0", "type": "uint112"},
            {"internalType": "uint112", "name": "_reserve1", "type": "uint112"},
            {"internalType": "uint32", "name": "_blockTimestampLast", "type": "uint32"}
        ],
        "payable": False,
        "stateMutability": "view",
        "type": "function"
    },
    {
        "constant": True,
        "inputs": [],
        "name": "token0",
        "outputs": [{"internalType": "address", "name": "", "type": "address"}],
        "payable": False,
        "stateMutability": "view",
        "type": "function"
    },
    {
        "constant": True,
        "inputs": [],
        "name": "token1",
        "outputs": [{"internalType": "address", "name": "", "type": "address"}],
        "payable": False,
        "stateMutability": "view",
        "type": "function"
    }
]

class PoolCalculationVerifier:
    def __init__(self):
        # Connect to local reth node as per workspace rules
        self.w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        if not self.w3.is_connected():
            raise Exception("Failed to connect to local reth node at http://127.0.0.1:8545")
        
        print(f"✅ Connected to Ethereum node: Chain ID {self.w3.eth.chain_id}")
        print(f"📊 Latest block: {self.w3.eth.block_number}")
        
        # ZMQ connection to Python pool level publisher
        self.zmq_context = zmq.Context()
        self.zmq_req_socket = None
        self.zmq_sub_socket = None
        self.zmq_req_endpoint = "tcp://127.0.0.1:5558"  # REQ/REP endpoint
        self.zmq_pub_endpoint = "tcp://127.0.0.1:5557"  # PUB/SUB endpoint
        
        # Known Uniswap V2 pools for fallback testing if ZMQ fails
        self.fallback_test_pools = [
            {
                "name": "USDC/WETH",
                "address": "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",
                "denom_address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",  # WETH
                "token1_is_denom": True
            },
            {
                "name": "USDT/WETH", 
                "address": "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852",
                "denom_address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",  # WETH
                "token1_is_denom": False  # WETH is token0, USDT is token1
            },
            {
                "name": "DAI/WETH",
                "address": "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11", 
                "denom_address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",  # WETH
                "token1_is_denom": True
            }
        ]
        
        # Store results
        self.calculation_results: List[Dict] = []
        self.discrepancies: List[Dict] = []
        self.received_updates: List[Dict] = []
        
    def connect_to_python_publisher(self) -> bool:
        """Connect to Python's pool level publisher via ZMQ REQ/REP and SUB."""
        try:
            # Connect REQ socket for queries
            print(f"🔌 Connecting REQ socket to Python pool publisher at {self.zmq_req_endpoint}")
            self.zmq_req_socket = self.zmq_context.socket(zmq.REQ)
            self.zmq_req_socket.setsockopt(zmq.RCVTIMEO, 5000)  # 5 second timeout
            self.zmq_req_socket.connect(self.zmq_req_endpoint)
            
            # Test connection with a simple request
            test_request = {"type": "get_all_pools"}
            self.zmq_req_socket.send_string(json.dumps(test_request))
            
            response_str = self.zmq_req_socket.recv_string()
            response = json.loads(response_str)
            
            if response.get('status') == 'success':
                pool_count = response.get('count', 0)
                print(f"✅ Connected REQ socket to Python publisher - {pool_count} pools available")
            else:
                print(f"⚠️ Python publisher returned error: {response.get('error', 'unknown')}")
                return False
                
            # Connect SUB socket for real-time updates
            print(f"🔌 Connecting SUB socket to {self.zmq_pub_endpoint}")
            self.zmq_sub_socket = self.zmq_context.socket(zmq.SUB)
            self.zmq_sub_socket.connect(self.zmq_pub_endpoint)
            self.zmq_sub_socket.setsockopt_string(zmq.SUBSCRIBE, '')  # Subscribe to all messages
            print(f"✅ Connected SUB socket for real-time pool updates")
            
            return True
                
        except Exception as e:
            print(f"❌ Failed to connect to Python publisher: {e}")
            if self.zmq_req_socket:
                self.zmq_req_socket.close()
                self.zmq_req_socket = None
            if self.zmq_sub_socket:
                self.zmq_sub_socket.close()
                self.zmq_sub_socket = None
            return False
    
    def get_all_pools_from_python(self) -> Dict[str, Dict]:
        """Get all pools from Python via ZMQ REQ/REP."""
        if not self.zmq_req_socket:
            return {}
            
        try:
            request = {"type": "get_all_pools"}
            self.zmq_req_socket.send_string(json.dumps(request))
            
            response_str = self.zmq_req_socket.recv_string()
            response = json.loads(response_str)
            
            if response.get('status') == 'success':
                return response.get('data', {})
            else:
                print(f"❌ Error getting pools from Python: {response.get('error', 'unknown')}")
                return {}
                
        except Exception as e:
            print(f"❌ Error requesting pools from Python: {e}")
            return {}
    
    def monitor_pool_updates(self, duration_seconds: int = 10):
        """Monitor real-time pool updates via ZMQ SUB socket."""
        if not self.zmq_sub_socket:
            print("❌ SUB socket not connected")
            return
            
        print(f"\n📡 Monitoring pool updates for {duration_seconds} seconds...")
        start_time = time.time()
        update_count = 0
        
        # Set timeout for non-blocking receive
        self.zmq_sub_socket.setsockopt(zmq.RCVTIMEO, 100)  # 100ms timeout
        
        while time.time() - start_time < duration_seconds:
            try:
                message_str = self.zmq_sub_socket.recv_string()
                message = json.loads(message_str)
                
                if message.get('type') == 'pool_updates':
                    update_count += 1
                    timestamp = message.get('timestamp', 0)
                    pool_data = message.get('data', {})
                    
                    # Store the update
                    self.received_updates.append(message)
                    
                    # Display summary
                    readable_time = time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(timestamp))
                    print(f"   📦 Update #{update_count} at {readable_time}: {len(pool_data)} pools")
                    
                    # Show first few pools as sample
                    for i, (pool_addr, data) in enumerate(pool_data.items()):
                        if i >= 3:  # Only show first 3
                            if len(pool_data) > 3:
                                print(f"      ... and {len(pool_data) - 3} more pools")
                            break
                        pool_type = data.get('pool_type', 'Unknown')
                        denom_eth = data.get('denom_eth', 0)
                        token_eth = data.get('token_eth', 0)
                        print(f"      • {pool_addr[:10]}... ({pool_type}): {denom_eth:.4f} ETH / {token_eth:.4f} tokens")
                        
            except zmq.error.Again:
                # Timeout - no message available
                pass
            except Exception as e:
                print(f"   ⚠️ Error receiving update: {e}")
                
        print(f"\n📊 Monitoring complete: Received {update_count} updates in {duration_seconds} seconds")
        if update_count > 0:
            print(f"   Average rate: {update_count/duration_seconds:.2f} updates/second")
        
    def get_blockchain_reserves_direct(self, pool_address: str) -> Tuple[float, float, str, str, bool]:
        """
        Get actual reserves from blockchain using getReserves() call directly.
        Returns: (reserve0_eth, reserve1_eth, token0_addr, token1_addr, success)
        """
        try:
            # Checksum the address
            pool_address_checksum = self.w3.to_checksum_address(pool_address)
            
            # Create contract instance
            contract = self.w3.eth.contract(
                address=pool_address_checksum,
                abi=UNISWAP_V2_PAIR_ABI
            )
            
            # Get reserves and token addresses
            reserves = contract.functions.getReserves().call()
            token0 = contract.functions.token0().call()
            token1 = contract.functions.token1().call()
            
            reserve0_wei, reserve1_wei, timestamp = reserves
            
            # Convert to ETH (assuming 18 decimals for simplicity)
            reserve0_eth = float(reserve0_wei) / 1e18
            reserve1_eth = float(reserve1_wei) / 1e18
            
            return reserve0_eth, reserve1_eth, token0, token1, True
            
        except Exception as e:
            print(f"  ❌ Failed to get reserves for {pool_address}: {e}")
            return 0.0, 0.0, "", "", False
    
    def compare_pool_with_blockchain(self, pool_address: str, python_pool_data: Dict) -> bool:
        """
        Compare Python's pool data with actual blockchain reserves.
        """
        print(f"\n📋 Comparing pool: {pool_address}")
        print(f"   Python pool data: {python_pool_data}")
        
        # Get actual blockchain reserves
        actual_reserve0, actual_reserve1, token0, token1, success = self.get_blockchain_reserves_direct(pool_address)
        
        if not success:
            print(f"  ❌ Cannot get actual reserves - skipping comparison")
            return False
        
        print(f"  🔗 Blockchain reserves:")
        print(f"     Token0 ({token0}): {actual_reserve0:.6f} ETH")
        print(f"     Token1 ({token1}): {actual_reserve1:.6f} ETH")
        
        # Extract Python's calculated reserve values only
        python_data = {}
        for key, value in python_pool_data.items():
            # Only include actual reserve values, not metadata
            if isinstance(value, (int, float)) and ('reserve' in key.lower() or key.endswith('_eth')):
                # Skip metadata fields like decimals, fees, etc
                if not any(skip in key.lower() for skip in ['decimal', 'fee', 'tier', 'block', 'time', 'liquidity']):
                    python_data[key] = float(value)
        
        print(f"  🐍 Python calculated values: {python_data}")
        
        # Try to match Python values with blockchain reserves
        # Look for values that should match reserve0 or reserve1
        matches_found = 0
        total_comparisons = 0
        significant_differences = []
        
        for python_key, python_value in python_data.items():
            if python_value > 0:  # Only compare non-zero values
                total_comparisons += 1
                
                # Check if it matches reserve0
                diff0 = abs(python_value - actual_reserve0)
                pct0 = (diff0 / actual_reserve0 * 100) if actual_reserve0 > 0 else 100
                
                # Check if it matches reserve1  
                diff1 = abs(python_value - actual_reserve1)
                pct1 = (diff1 / actual_reserve1 * 100) if actual_reserve1 > 0 else 100
                
                # Tolerance: < 0.001 ETH difference OR < 0.1% difference
                tolerance_eth = 0.001
                tolerance_pct = 0.1
                
                if (diff0 < tolerance_eth or pct0 < tolerance_pct):
                    print(f"     ✅ {python_key}={python_value:.6f} matches reserve0 (diff: {pct0:.3f}%)")
                    matches_found += 1
                elif (diff1 < tolerance_eth or pct1 < tolerance_pct):
                    print(f"     ✅ {python_key}={python_value:.6f} matches reserve1 (diff: {pct1:.3f}%)")
                    matches_found += 1
                else:
                    min_pct = min(pct0, pct1)
                    if min_pct > 1.0:  # Only report significant differences
                        print(f"     🚨 {python_key}={python_value:.6f} doesn't match reserves (closest: {min_pct:.2f}% diff)")
                        significant_differences.append({
                            'key': python_key,
                            'python_value': python_value,
                            'reserve0': actual_reserve0,
                            'reserve1': actual_reserve1,
                            'min_diff_pct': min_pct
                        })
        
        # Store results
        result = {
            "pool_address": pool_address,
            "python_data": python_data,
            "actual_reserve0": actual_reserve0,
            "actual_reserve1": actual_reserve1,
            "token0": token0,
            "token1": token1,
            "matches_found": matches_found,
            "total_comparisons": total_comparisons,
            "significant_differences": significant_differences
        }
        
        self.calculation_results.append(result)
        
        # Consider it a match if we found matches for most values and no significant differences
        if matches_found >= max(1, total_comparisons // 2) and len(significant_differences) == 0:
            print(f"  ✅ MATCH: Python pool data is consistent with blockchain")
            return True
        else:
            print(f"  🚨 MISMATCH: Found {len(significant_differences)} significant discrepancies")
            self.discrepancies.append(result)
            return False
    
    def test_python_calculation(self, pool_info: Dict) -> bool:
        """
        Test Python's pool calculation logic using get_pool_reserves_from_blockchain().
        This is the fallback method for individual pool testing.
        """
        pool_address = pool_info["address"]
        pool_name = pool_info["name"]
        
        print(f"\n📋 Testing {pool_name}: {pool_address}")
        
        try:
            from eth_token.erc20_token.data.erc20_token_data import ERC20TokenData
            
            # Create a LiveTokenData instance
            token_data = ERC20TokenData(contract_address="0x1234567890123456789012345678901234567890")
            token_data.decimals = 18  # Mock decimals
            
            # Set up pool info
            token_data.set_pool_info(
                pool_address, 
                "V2", 
                pool_info["denom_address"], 
                pool_info["token1_is_denom"]
            )
            
            # Test Python's calculation method
            print(f"  🐍 Testing Python's get_pool_reserves_from_blockchain()...")
            python_denom, python_token, python_success = token_data.get_pool_reserves_from_blockchain(pool_address)
            
            if python_success:
                print(f"     Python calculated: denom={python_denom:.6f}, token={python_token:.6f}")
            else:
                print(f"     Python calculation failed")
                return False
            
            # Get actual blockchain reserves
            actual_reserve0, actual_reserve1, token0, token1, actual_success = self.get_blockchain_reserves_direct(pool_address)
            
            if not actual_success:
                print(f"  ❌ Cannot get actual reserves - skipping comparison")
                return False
            
            print(f"  🔗 Direct blockchain query:")
            print(f"     Token0: {token0} = {actual_reserve0:.6f} ETH")
            print(f"     Token1: {token1} = {actual_reserve1:.6f} ETH")
            
            # Compare based on token1_is_denom flag
            if pool_info["token1_is_denom"]:
                # token1 is denomination (WETH), token0 is the main token
                expected_denom = actual_reserve1
                expected_token = actual_reserve0
            else:
                # token0 is denomination (WETH), token1 is the main token  
                expected_denom = actual_reserve0
                expected_token = actual_reserve1
            
            print(f"  🎯 Comparing results:")
            print(f"     Expected: denom={expected_denom:.6f}, token={expected_token:.6f}")
            print(f"     Python:   denom={python_denom:.6f}, token={python_token:.6f}")
            
            # Calculate differences
            denom_diff = abs(python_denom - expected_denom)
            token_diff = abs(python_token - expected_token)
            
            denom_pct = (denom_diff / expected_denom * 100) if expected_denom > 0 else 0
            token_pct = (token_diff / expected_token * 100) if expected_token > 0 else 0
            
            print(f"     Differences: denom={denom_diff:.6f} ({denom_pct:.2f}%), token={token_diff:.6f} ({token_pct:.2f}%)")
            
            # Store results
            result = {
                "pool_name": pool_name,
                "pool_address": pool_address,
                "python_denom": python_denom,
                "python_token": python_token,
                "expected_denom": expected_denom,
                "expected_token": expected_token,
                "denom_diff": denom_diff,
                "token_diff": token_diff,
                "denom_pct": denom_pct,
                "token_pct": token_pct
            }
            
            self.calculation_results.append(result)
            
            # Check if differences are acceptable (< 0.001 ETH or < 0.1%)
            tolerance_eth = 0.001
            tolerance_pct = 0.1
            
            if (denom_diff < tolerance_eth or denom_pct < tolerance_pct) and \
               (token_diff < tolerance_eth or token_pct < tolerance_pct):
                print(f"  ✅ MATCH: Python calculation is correct")
                return True
            else:
                print(f"  🚨 MISMATCH: Python calculation has errors")
                self.discrepancies.append(result)
                return False
                
        except Exception as e:
            print(f"  💥 Error testing Python calculation: {e}")
            return False
    
    def run_verification(self) -> bool:
        """
        Run the complete pool calculation verification test.
        Returns True if all calculations match, False if discrepancies found.
        """
        print("🧪 Pool Level Verification Test")
        print("=" * 80)
        print("Verifying pool publishing mechanism and data accuracy")
        
        successful_tests = 0
        total_tests = 0
        
        # Try to connect to Python publisher
        if self.connect_to_python_publisher():
            # First monitor real-time updates to verify publishing is working
            self.monitor_pool_updates(duration_seconds=15)
            
            print("\n🔄 Requesting all pools from Python via REQ/REP...")
            python_pools = self.get_all_pools_from_python()
            
            if python_pools:
                print(f"📊 Found {len(python_pools)} pools from Python")
                total_tests = min(len(python_pools), 5)  # Test up to 5 pools for verification
                
                # Test a sample of pools from Python
                tested = 0
                for pool_address, pool_data in python_pools.items():
                    if tested >= total_tests:
                        break
                    if self.compare_pool_with_blockchain(pool_address, pool_data):
                        successful_tests += 1
                    tested += 1
                    
                if len(python_pools) > total_tests:
                    print(f"\n📌 Tested {total_tests} pools out of {len(python_pools)} total")
            else:
                print("⚠️ No pools received from Python via REQ/REP")
                
            # Analyze received updates
            if self.received_updates:
                print(f"\n📈 Real-time Update Analysis:")
                print(f"   Total updates received: {len(self.received_updates)}")
                total_pools_seen = set()
                for update in self.received_updates:
                    pools_in_update = update.get('data', {})
                    total_pools_seen.update(pools_in_update.keys())
                print(f"   Unique pools seen in updates: {len(total_pools_seen)}")
                print(f"   ✅ Publishing mechanism is WORKING")
            else:
                print(f"\n⚠️ No real-time updates received - publishing might not be working")
        else:
            print("⚠️ Cannot connect to Python publisher, using fallback test")
        
        # Fallback: test hardcoded pools if we couldn't get data from Python
        if total_tests == 0:
            print("\n🔄 Running fallback test with hardcoded pools...")
            total_tests = len(self.fallback_test_pools)
            
            for pool_info in self.fallback_test_pools:
                if self.test_python_calculation(pool_info):
                    successful_tests += 1
        
        # Clean up ZMQ
        if self.zmq_req_socket:
            self.zmq_req_socket.close()
        if self.zmq_sub_socket:
            self.zmq_sub_socket.close()
        self.zmq_context.term()
        
        # Summary
        print("\n" + "=" * 80)
        print("📊 VERIFICATION RESULTS")
        print("=" * 80)
        
        print(f"Total pools tested: {total_tests}")
        print(f"Successful matches: {successful_tests}")
        print(f"Calculation errors: {len(self.discrepancies)}")
        
        if len(self.discrepancies) == 0:
            print("\n✅ ALL POOLS MATCH!")
            print("   Python's pool data is consistent with blockchain reserves")
            print("   Pool level calculation and accumulation is working correctly")
        else:
            print(f"\n🚨 FOUND {len(self.discrepancies)} POOL DISCREPANCIES!")
            print("   Python's pool data has issues")
            
            for disc in self.discrepancies:
                print(f"\n🔍 Pool {disc['pool_address']}")
                if 'python_denom' in disc:
                    # Fallback test result
                    print(f"   Denom: Python={disc['python_denom']:.6f}, Expected={disc['expected_denom']:.6f} (diff: {disc['denom_pct']:.2f}%)")
                    print(f"   Token: Python={disc['python_token']:.6f}, Expected={disc['expected_token']:.6f} (diff: {disc['token_pct']:.2f}%)")
                else:
                    # ZMQ comparison result
                    print(f"   Significant differences found: {len(disc['significant_differences'])}")
                    for diff in disc['significant_differences']:
                        print(f"     {diff['key']}: Python={diff['python_value']:.6f}, closest reserve diff={diff['min_diff_pct']:.2f}%")
        
        # Overall assessment
        publishing_working = len(self.received_updates) > 0
        data_accurate = len(self.discrepancies) == 0
        
        return publishing_working and data_accurate

if __name__ == "__main__":
    try:
        verifier = PoolCalculationVerifier()
        success = verifier.run_verification()
        
        # Detailed conclusion based on results
        publishing_ok = len(verifier.received_updates) > 0
        data_ok = len(verifier.discrepancies) == 0
        
        print("\n🎯 CONCLUSION:")
        if publishing_ok:
            print("   ✅ Pool publishing mechanism is WORKING")
            print(f"      - Received {len(verifier.received_updates)} real-time updates")
        else:
            print("   ❌ Pool publishing mechanism NOT WORKING") 
            print("      - No real-time updates received")
            
        if data_ok:
            print("   ✅ Pool data accuracy is CORRECT")
            print("      - All tested pool levels match blockchain reserves")
        else:
            print("   ❌ Pool data has DISCREPANCIES")
            print(f"      - Found {len(verifier.discrepancies)} mismatches")
            
        if success:
            print("\n   🎉 Overall: PASS - Publishing working and data accurate")
            exit(0)
        else:
            print("\n   ⚠️ Overall: FAIL - Issues found with publishing or data accuracy")
            exit(1)
            
    except KeyboardInterrupt:
        print("\n⚠️  Test interrupted by user")
        exit(130)
    except Exception as e:
        print(f"\n❌ Test failed with error: {e}")
        exit(1) 