#!/usr/bin/env python3
"""
Real Scam Detection Test Runner

Runs scam detection tests using real blockchain data.
No mocks - only real chain interactions.
"""

import asyncio
import sys
from web3 import Web3

# Add paths
sys.path.append('/home/nima/code/crypto/py')

from tests.live_tokens.test_scam_detection_integration import TestScamDetectionIntegration
from eth_token.utils.logger import get_logger


async def main():
    """Run real blockchain scam detection tests"""
    print("🚀 Starting Real Blockchain Scam Detection Tests")
    print("=" * 60)
    
    # Initialize components
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    logger = get_logger("scam_detection_test", log_folder="tests")
    
    # Check connection
    if not w3.is_connected():
        print("❌ Cannot connect to Ethereum node at http://127.0.0.1:8545")
        return
        
    print(f"✅ Connected to Ethereum node (block {w3.eth.block_number})")
    
    # Initialize test class
    test_instance = TestScamDetectionIntegration()
    
    # Test tokens to analyze
    test_tokens = [
        ("0x1b5F55aacB562236DBB31A3644Ad80F0dA98b73F", "Test Token"),
        ("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "WETH"),
        # Add more real token addresses to test
    ]
    
    print(f"\n🔍 Testing {len(test_tokens)} tokens...")
    
    results = []
    for token_address, token_name in test_tokens:
        print(f"\n{'='*20}")
        print(f"Testing: {token_name}")
        print(f"Address: {token_address}")
        print(f"{'='*20}")
        
        try:
            # Build token from real blockchain data
            live_token = await test_instance.build_live_token_from_chain(
                token_address, w3, logger
            )
            
            if live_token:
                result = {
                    'address': token_address,
                    'name': token_name,
                    'token_name': live_token.token_data.name,
                    'symbol': live_token.token_data.symbol,
                    'has_pools': live_token.token_data.has_pool,
                    'pool_count': len(live_token.token_data.pool_addresses) if live_token.token_data.pool_addresses else 0,
                    'is_scam': live_token.token_data.is_scam,
                    'scam_reason': live_token.token_data.scam_label,
                    'total_supply': live_token.token_data.total_supply
                }
                results.append(result)
                
                # Detailed analysis if pools exist
                if live_token.token_data.has_pool and live_token.token_data.pool_manager:
                    print(f"\n📊 Pool Analysis:")
                    pool_manager = live_token.token_data.pool_manager
                    
                    for pool_address in live_token.token_data.pool_addresses:
                        pool = pool_manager.get_pool(pool_address)
                        if pool:
                            print(f"   Pool: {pool_address[:10]}...")
                            print(f"      Protocol: {pool.get_protocol()}")
                            print(f"      Healthy: {pool.is_healthy()}")
                            print(f"      Denom Reserve: {pool.get_denom_reserve()}")
                            print(f"      Token Reserve: {pool.get_token_reserve()}")
                            print(f"      Price: {pool.get_price()}")
                    
                    # Pool manager stats
                    stats = pool_manager.get_stats()
                    print(f"\n📈 Stats: {stats['total_pools']} pools, {stats['healthy_pools']} healthy")
                
            else:
                print("❌ Could not build token from blockchain data")
                
        except Exception as e:
            print(f"❌ Error: {e}")
    
    # Summary
    print(f"\n{'='*60}")
    print("📋 FINAL RESULTS")
    print(f"{'='*60}")
    
    for result in results:
        status = "🚨 SCAM" if result['is_scam'] else "✅ SAFE"
        pools = f"{result['pool_count']} pools" if result['pool_count'] > 0 else "No pools"
        
        print(f"{status} {result['symbol']} ({result['token_name']}) - {pools}")
        if result['is_scam']:
            print(f"     Reason: {result['scam_reason']}")
    
    print(f"\n✅ Analysis complete! Tested {len(results)} tokens.")


if __name__ == "__main__":
    asyncio.run(main())