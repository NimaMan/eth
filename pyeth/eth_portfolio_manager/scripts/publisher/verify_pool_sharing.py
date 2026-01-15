#!/usr/bin/env python3
"""
Comprehensive Pool Sharing Verification Script

OBJECTIVE:
----------
Verify that the pool levels shared by the Python TokenInfoPublisher exactly match
the actual on-chain pool reserves for ETH-denominated pools.

WHAT THIS SCRIPT DOES:
---------------------
1. Connects to the Python TokenInfoPublisher via ZMQ (ports 5557/5558)
2. Retrieves all pools being shared by the publisher
3. For each pool:
   - Verifies it's an ETH-denominated pool (WETH as denomination token)
   - Queries the blockchain directly for actual reserves
   - Compares published eth_reserve with actual WETH reserve
   - Compares published token_reserve with actual token reserve
   - Flags any discrepancies beyond acceptable tolerance

VERIFICATION CRITERIA:
--------------------
- Pool must have WETH as one of the tokens
- Published eth_reserve must match actual WETH reserve (±0.1% or ±0.001 ETH)
- Published token_reserve must match actual token reserve (±0.1%)
- All non-ETH pools should be filtered out (not published)

OUTPUT:
-------
- Summary of all pools tested
- List of any pools with mismatched reserves
- Confirmation that only ETH pools are being shared
- Real-time update monitoring to verify publishing mechanism

This is the ONLY pool verification script needed. All others should be removed.
"""

import json
import time
import zmq
from web3 import Web3
from typing import Dict, Tuple, List, Optional, Set
import asyncio
from datetime import datetime

# Constants
WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
TOLERANCE_PERCENT = 0.1  # 0.1% tolerance
TOLERANCE_ETH_ABS = 0.001  # 0.001 ETH absolute tolerance

# Standard ERC20 ABI for decimals and symbol
ERC20_ABI = [
    {
        "constant": True,
        "inputs": [],
        "name": "decimals",
        "outputs": [{"name": "", "type": "uint8"}],
        "type": "function"
    },
    {
        "constant": True,
        "inputs": [],
        "name": "symbol",
        "outputs": [{"name": "", "type": "string"}],
        "type": "function"
    }
]

# Uniswap V2 Pair ABI
UNISWAP_V2_PAIR_ABI = [
    {
        "constant": True,
        "inputs": [],
        "name": "getReserves",
        "outputs": [
            {"name": "_reserve0", "type": "uint112"},
            {"name": "_reserve1", "type": "uint112"},
            {"name": "_blockTimestampLast", "type": "uint32"}
        ],
        "type": "function"
    },
    {
        "constant": True,
        "inputs": [],
        "name": "token0",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    },
    {
        "constant": True,
        "inputs": [],
        "name": "token1", 
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    }
]


class PoolSharingVerifier:
    def __init__(self):
        # Connect to local reth node
        self.w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        if not self.w3.is_connected():
            raise Exception("Failed to connect to local reth node")
        
        print(f"✅ Connected to Ethereum node (Chain ID: {self.w3.eth.chain_id})")
        print(f"📊 Latest block: {self.w3.eth.block_number:,}")
        
        # ZMQ setup
        self.zmq_context = zmq.Context()
        self.zmq_req_socket = None
        self.zmq_sub_socket = None
        
        # Cache for token info
        self.token_decimals_cache = {}
        self.token_symbol_cache = {}
        
        # Results tracking
        self.results = {
            'total_pools': 0,
            'verified_pools': 0,
            'failed_pools': [],
            'non_eth_pools': [],
            'updates_received': 0,
            'start_time': datetime.now()
        }
    
    def get_token_info(self, token_address: str) -> Tuple[int, str]:
        """Get decimals and symbol for a token."""
        if token_address in self.token_decimals_cache:
            return self.token_decimals_cache[token_address], self.token_symbol_cache[token_address]
        
        try:
            token_contract = self.w3.eth.contract(
                address=self.w3.to_checksum_address(token_address),
                abi=ERC20_ABI
            )
            decimals = token_contract.functions.decimals().call()
            symbol = token_contract.functions.symbol().call()
            
            self.token_decimals_cache[token_address] = decimals
            self.token_symbol_cache[token_address] = symbol
            
            return decimals, symbol
        except:
            # Default for tokens that don't implement decimals/symbol
            self.token_decimals_cache[token_address] = 18
            self.token_symbol_cache[token_address] = "UNKNOWN"
            return 18, "UNKNOWN"
    
    def get_pool_reserves_from_chain(self, pool_address: str) -> Optional[Dict]:
        """Get actual pool reserves from blockchain."""
        try:
            pool_contract = self.w3.eth.contract(
                address=self.w3.to_checksum_address(pool_address),
                abi=UNISWAP_V2_PAIR_ABI
            )
            
            # Get token addresses
            token0 = pool_contract.functions.token0().call()
            token1 = pool_contract.functions.token1().call()
            
            # Get reserves
            reserves = pool_contract.functions.getReserves().call()
            reserve0_raw, reserve1_raw, _ = reserves
            
            # Get token info
            decimals0, symbol0 = self.get_token_info(token0)
            decimals1, symbol1 = self.get_token_info(token1)
            
            # Convert to human-readable
            reserve0 = float(reserve0_raw) / (10 ** decimals0)
            reserve1 = float(reserve1_raw) / (10 ** decimals1)
            
            # Determine which is WETH
            token0_is_weth = token0.lower() == WETH_ADDRESS.lower()
            token1_is_weth = token1.lower() == WETH_ADDRESS.lower()
            
            # One token must be WETH for ETH pools
            if not (token0_is_weth or token1_is_weth):
                return {
                    'is_eth_pool': False,
                    'token0': token0,
                    'token1': token1,
                    'symbol0': symbol0,
                    'symbol1': symbol1
                }
            
            # Organize data with WETH as denomination
            if token0_is_weth:
                eth_reserve = reserve0
                token_reserve = reserve1
                token_address = token1
                token_symbol = symbol1
            else:
                eth_reserve = reserve1
                token_reserve = reserve0
                token_address = token0
                token_symbol = symbol0
            
            return {
                'is_eth_pool': True,
                'eth_reserve': eth_reserve,
                'token_reserve': token_reserve,
                'token_address': token_address,
                'token_symbol': token_symbol,
                'token0': token0,
                'token1': token1,
                'reserve0': reserve0,
                'reserve1': reserve1
            }
            
        except Exception as e:
            print(f"❌ Error getting pool reserves for {pool_address}: {e}")
            return None
    
    def verify_pool(self, pool_address: str, published_data: Dict) -> bool:
        """Verify a single pool's published data against blockchain."""
        # Get actual reserves from chain
        chain_data = self.get_pool_reserves_from_chain(pool_address)
        
        if not chain_data:
            print(f"\n❌ {pool_address}: Failed to get chain data")
            self.results['failed_pools'].append({
                'pool': pool_address,
                'reason': 'Failed to query blockchain'
            })
            return False
        
        # Check if it's an ETH pool
        if not chain_data['is_eth_pool']:
            print(f"\n⚠️  {pool_address}: Not an ETH pool! ({chain_data['symbol0']}/{chain_data['symbol1']})")
            self.results['non_eth_pools'].append({
                'pool': pool_address,
                'tokens': f"{chain_data['symbol0']}/{chain_data['symbol1']}"
            })
            self.results['failed_pools'].append({
                'pool': pool_address,
                'reason': 'Not an ETH pool but was published'
            })
            return False
        
        # Extract published values
        pub_eth = published_data.get('eth_reserve', 0)
        pub_token = published_data.get('token_reserve', 0)
        pub_token_addr = published_data.get('token_address', '').lower()
        
        # Verify token address matches
        if pub_token_addr != chain_data['token_address'].lower():
            print(f"\n❌ {pool_address}: Token address mismatch!")
            print(f"   Published: {pub_token_addr}")
            print(f"   Chain:     {chain_data['token_address'].lower()}")
            self.results['failed_pools'].append({
                'pool': pool_address,
                'reason': 'Token address mismatch'
            })
            return False
        
        # Compare reserves
        eth_diff = abs(pub_eth - chain_data['eth_reserve'])
        eth_pct = (eth_diff / chain_data['eth_reserve'] * 100) if chain_data['eth_reserve'] > 0 else 0
        
        token_diff = abs(pub_token - chain_data['token_reserve'])
        token_pct = (token_diff / chain_data['token_reserve'] * 100) if chain_data['token_reserve'] > 0 else 0
        
        # Check tolerances
        eth_ok = eth_pct <= TOLERANCE_PERCENT or eth_diff <= TOLERANCE_ETH_ABS
        token_ok = token_pct <= TOLERANCE_PERCENT
        
        if eth_ok and token_ok:
            print(f"\n✅ {pool_address}: WETH/{chain_data['token_symbol']}")
            print(f"   ETH:   {pub_eth:.6f} (diff: {eth_pct:.3f}%)")
            print(f"   Token: {pub_token:.2f} (diff: {token_pct:.3f}%)")
            return True
        else:
            print(f"\n❌ {pool_address}: WETH/{chain_data['token_symbol']} - MISMATCH")
            print(f"   ETH Reserve:")
            print(f"     Published: {pub_eth:.6f}")
            print(f"     Chain:     {chain_data['eth_reserve']:.6f}")
            print(f"     Diff:      {eth_diff:.6f} ({eth_pct:.2f}%)")
            print(f"   Token Reserve:")
            print(f"     Published: {pub_token:.2f}")
            print(f"     Chain:     {chain_data['token_reserve']:.2f}")
            print(f"     Diff:      {token_diff:.2f} ({token_pct:.2f}%)")
            
            self.results['failed_pools'].append({
                'pool': pool_address,
                'token_symbol': chain_data['token_symbol'],
                'eth_diff_pct': eth_pct,
                'token_diff_pct': token_pct,
                'reason': 'Reserve mismatch'
            })
            return False
    
    def connect_to_publisher(self) -> bool:
        """Connect to TokenInfoPublisher via ZMQ."""
        try:
            # REQ socket for queries
            self.zmq_req_socket = self.zmq_context.socket(zmq.REQ)
            self.zmq_req_socket.setsockopt(zmq.RCVTIMEO, 5000)
            self.zmq_req_socket.connect("tcp://127.0.0.1:5558")
            
            # Test connection
            self.zmq_req_socket.send_string(json.dumps({"type": "get_pool_stats"}))
            response = json.loads(self.zmq_req_socket.recv_string())
            
            if response.get('status') == 'success':
                stats = response.get('stats', {})
                print(f"\n✅ Connected to TokenInfoPublisher")
                print(f"   Pools tracked: {stats.get('pool_count', 0)}")
                print(f"   ETH threshold: {stats.get('eth_threshold', 0)} ETH")
                print(f"   Total ETH: {stats.get('total_eth_locked', 0):.2f} ETH")
            
            # SUB socket for updates
            self.zmq_sub_socket = self.zmq_context.socket(zmq.SUB)
            self.zmq_sub_socket.connect("tcp://127.0.0.1:5557")
            self.zmq_sub_socket.setsockopt_string(zmq.SUBSCRIBE, '')
            
            return True
            
        except Exception as e:
            print(f"❌ Failed to connect to publisher: {e}")
            return False
    
    def monitor_updates(self, duration: int = 15):
        """Monitor real-time pool updates (default 15s to catch at least one block)."""
        if not self.zmq_sub_socket:
            return
        
        print(f"\n📡 Monitoring pool updates for {duration} seconds (blocks arrive every ~12s)...")
        start = time.time()
        last_dot = time.time()
        
        self.zmq_sub_socket.setsockopt(zmq.RCVTIMEO, 100)
        
        while time.time() - start < duration:
            try:
                msg = self.zmq_sub_socket.recv_string()
                data = json.loads(msg)
                
                if data.get('type') == 'token_updates':
                    self.results['updates_received'] += 1
                    pools = data.get('data', {})
                    
                    print(f"\n   📦 Update #{self.results['updates_received']} received at {time.time() - start:.1f}s")
                    print(f"      Pools in update: {len(pools)}")
                    
                    # Sample first pool to check
                    if pools and self.results['updates_received'] == 1:
                        sample_addr = list(pools.keys())[0]
                        sample_data = pools[sample_addr]
                        print(f"      Sample pool: {sample_addr[:10]}...")
                        print(f"      ETH Reserve: {sample_data.get('eth_reserve', 0):.4f}")
                        print(f"      Token Reserve: {sample_data.get('token_reserve', 0):.2f}")
                        
            except zmq.error.Again:
                # Print progress dots
                if time.time() - last_dot > 1:
                    print(".", end="", flush=True)
                    last_dot = time.time()
            except Exception as e:
                print(f"\n   ⚠️ Error: {e}")
        
        print(f"\n   Total updates received: {self.results['updates_received']}")
    
    def run_verification(self):
        """Run complete pool sharing verification."""
        print("\n" + "="*80)
        print("🔍 POOL SHARING VERIFICATION")
        print("="*80)
        
        if not self.connect_to_publisher():
            return False
        
        # Monitor updates for at least one block period
        self.monitor_updates(15)
        
        # Get all pools
        print("\n📋 Requesting all pools from publisher...")
        self.zmq_req_socket.send_string(json.dumps({"type": "get_all_pools"}))
        response = json.loads(self.zmq_req_socket.recv_string())
        
        if response.get('status') != 'success':
            print("❌ Failed to get pools from publisher")
            return False
        
        pools = response.get('data', {})
        self.results['total_pools'] = len(pools)
        print(f"Received {len(pools)} pools to verify")
        
        if len(pools) == 0:
            print("\n⚠️  No pools to verify. Is the publisher running?")
            return False
        
        # Verify each pool
        print("\n" + "-"*80)
        print("VERIFYING INDIVIDUAL POOLS:")
        print("-"*80)
        
        for pool_addr, pool_data in pools.items():
            if self.verify_pool(pool_addr, pool_data):
                self.results['verified_pools'] += 1
        
        # Print summary
        self.print_summary()
        
        # Cleanup
        if self.zmq_req_socket:
            self.zmq_req_socket.close()
        if self.zmq_sub_socket:
            self.zmq_sub_socket.close()
        self.zmq_context.term()
        
        return len(self.results['failed_pools']) == 0
    
    def print_summary(self):
        """Print verification summary."""
        print("\n" + "="*80)
        print("📊 VERIFICATION SUMMARY")
        print("="*80)
        
        print(f"\nTotal pools checked: {self.results['total_pools']}")
        print(f"Successfully verified: {self.results['verified_pools']}")
        print(f"Failed verification: {len(self.results['failed_pools'])}")
        
        # Publishing status
        if self.results['updates_received'] > 0:
            print(f"\n✅ Real-time publishing: WORKING ({self.results['updates_received']} updates received)")
        else:
            print("\n⚠️  Real-time publishing: No updates received (may need to wait for next block)")
        
        # Pool filtering status
        if self.results['non_eth_pools']:
            print(f"\n❌ ETH-only filtering: FAILED")
            print(f"   Found {len(self.results['non_eth_pools'])} non-ETH pools being published:")
            for pool in self.results['non_eth_pools'][:5]:
                print(f"   • {pool['pool']}: {pool['tokens']}")
        else:
            print("\n✅ ETH-only filtering: WORKING (all pools have WETH)")
        
        # Failed pools details
        if self.results['failed_pools']:
            print(f"\n❌ FAILED POOLS ({len(self.results['failed_pools'])}):")
            for failure in self.results['failed_pools']:
                print(f"\n   Pool: {failure['pool']}")
                print(f"   Reason: {failure['reason']}")
                if 'eth_diff_pct' in failure:
                    print(f"   ETH diff: {failure['eth_diff_pct']:.2f}%")
                    print(f"   Token diff: {failure['token_diff_pct']:.2f}%")
        
        # Overall result
        print("\n" + "="*80)
        
        # Determine pass/fail based on critical criteria
        bulk_retrieval_works = self.results['total_pools'] > 0
        data_accuracy_ok = len(self.results['failed_pools']) == 0
        eth_filtering_ok = len(self.results['non_eth_pools']) == 0
        
        if bulk_retrieval_works and data_accuracy_ok and eth_filtering_ok:
            print("✅ OVERALL: PASS - Pool sharing is working correctly!")
            print("   • Bulk retrieval via get_all_pools: ✓")
            print("   • Pool data accuracy: ✓")
            print("   • ETH-only filtering: ✓")
            if self.results['updates_received'] == 0:
                print("   • Real-time updates: ⚠️ (no updates in monitoring window)")
        else:
            print("❌ OVERALL: FAIL - Critical issues found:")
            if not bulk_retrieval_works:
                print("   • Bulk retrieval: ✗ (no pools returned)")
            if not data_accuracy_ok:
                print("   • Pool data accuracy: ✗ (mismatches found)")
            if not eth_filtering_ok:
                print("   • ETH-only filtering: ✗ (non-ETH pools published)")
        print("="*80)


def main():
    """Main entry point."""
    try:
        verifier = PoolSharingVerifier()
        success = verifier.run_verification()
        
        if success:
            print("\n🎉 Pool sharing verification PASSED!")
            exit(0)
        else:
            print("\n⚠️  Pool sharing verification FAILED!")
            exit(1)
            
    except KeyboardInterrupt:
        print("\n⚠️ Verification interrupted by user")
        exit(130)
    except Exception as e:
        print(f"\n❌ Error during verification: {e}")
        import traceback
        traceback.print_exc()
        exit(1)


if __name__ == "__main__":
    main()