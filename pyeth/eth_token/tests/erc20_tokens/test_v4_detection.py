#!/usr/bin/env python3
"""
Test V4 pool detection and validation
"""

from web3 import Web3

def test_v4_pool_existence():
    """Test if we can detect V4 pools for CABAL token."""
    
    print("="*80)
    print("V4 Pool Detection Test")
    print("="*80)
    
    # Setup
    w3 = Web3(Web3.HTTPProvider("http://localhost:8545"))
    if not w3.is_connected():
        print("❌ Failed to connect to Ethereum node")
        return
        
    print("✅ Connected to Ethereum node")
    
    token_address = w3.to_checksum_address("0x62Ca69ac5e15f472918729e628935304C81caba1")
    weth_address = w3.to_checksum_address("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
    v4_pool_manager = w3.to_checksum_address("0x000000000004444c5dc75cb358380d2e3de08a90")
    
    print(f"\n1. Testing V4 Pool Manager Contract:")
    print(f"   Address: {v4_pool_manager}")
    
    # Check if contract exists
    code = w3.eth.get_code(v4_pool_manager)
    if code == b'':
        print("   ❌ No contract found at Pool Manager address")
        return
    else:
        print("   ✅ Pool Manager contract exists")
    
    print(f"\n2. Testing Token Contract:")
    print(f"   CABAL: {token_address}")
    
    token_code = w3.eth.get_code(token_address)
    if token_code == b'':
        print("   ❌ No contract found at token address")
        return
    else:
        print("   ✅ Token contract exists")
    
    # Test the transaction hash provided by user
    tx_hash = "0xa43f445b1c97b8a4b0b5bb6c6a47b8b6fe01d2db0a5a78b9b94e6b3fe9ac5b8d"
    
    print(f"\n3. Looking for V4 activity in recent blocks...")
    
    # Get latest block and search backwards for V4 events
    latest_block = w3.eth.get_block('latest')
    current_block = latest_block.number
    
    print(f"   Starting from block: {current_block}")
    
    # V4 event signatures
    v4_event_signatures = {
        '0x40e9cecb9f5f1f1c5b9c97dec2917b7ee92e57ba5563708daca94dd84ad7112f': 'BalanceDelta',
        '0xfb3b14c492e4b6b8bbc1a4dd949d3f6ad64b7c5e3b1e9eb7f11e7b1e3b3f1b3f': 'Initialize',  # Approximate
        '0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1': 'Swap',  # Approximate
    }
    
    v4_events_found = 0
    blocks_searched = 0
    max_blocks_to_search = 100
    
    for block_num in range(current_block, current_block - max_blocks_to_search, -1):
        try:
            block = w3.eth.get_block(block_num, full_transactions=True)
            blocks_searched += 1
            
            for tx in block.transactions:
                # Check if transaction interacted with Pool Manager
                if tx.to and tx.to.lower() == v4_pool_manager.lower():
                    print(f"   📍 Found V4 transaction: {tx.hash.hex()} in block {block_num}")
                    
                    # Get transaction receipt to check logs
                    receipt = w3.eth.get_transaction_receipt(tx.hash)
                    
                    for log in receipt.logs:
                        # Check if log is from Pool Manager
                        if log.address.lower() == v4_pool_manager.lower():
                            # Check if it's a known V4 event
                            if log.topics and len(log.topics) > 0:
                                event_sig = log.topics[0].hex()
                                if event_sig in v4_event_signatures:
                                    v4_events_found += 1
                                    print(f"      🎯 V4 {v4_event_signatures[event_sig]} event found!")
                                    
                                    # If it's a BalanceDelta, try to extract pool ID
                                    if event_sig == '0x40e9cecb9f5f1f1c5b9c97dec2917b7ee92e57ba5563708daca94dd84ad7112f':
                                        if len(log.topics) > 1:
                                            pool_id = log.topics[1].hex()
                                            print(f"         Pool ID: {pool_id}")
                                            
                                            # Check if this involves our token
                                            if len(log.topics) > 2:
                                                settler = log.topics[2].hex()
                                                print(f"         Settler: 0x{settler[-40:]}")
                                else:
                                    print(f"      Unknown V4 event: {event_sig}")
            
            if v4_events_found > 0:
                break
                
        except Exception as e:
            print(f"   Error processing block {block_num}: {e}")
            continue
    
    print(f"\n4. Search Results:")
    print(f"   - Blocks searched: {blocks_searched}")
    print(f"   - V4 events found: {v4_events_found}")
    
    if v4_events_found > 0:
        print("   ✅ V4 pools are active on this network")
        
        # Try to find pools involving our token
        print(f"\n5. Searching for CABAL token involvement...")
        
        # This would require more detailed analysis of events
        # For now, let's test if we can call any Pool Manager functions
        test_pool_manager_functions(w3, v4_pool_manager)
        
    else:
        print("   ⚠️  No V4 events found in recent blocks")
        print("   This might indicate:")
        print("     - V4 not deployed on this network")
        print("     - Different Pool Manager address")
        print("     - No recent V4 activity")

def test_pool_manager_functions(w3, pool_manager_address):
    """Test various Pool Manager function calls."""
    
    print(f"\n6. Testing Pool Manager Functions...")
    
    # Test different ABI approaches
    abis_to_test = [
        # Minimal ABI
        [
            {
                "inputs": [{"type": "bytes32", "name": "poolId"}],
                "name": "getSlot0",
                "outputs": [
                    {"type": "uint160", "name": "sqrtPriceX96"},
                    {"type": "int24", "name": "tick"}
                ],
                "stateMutability": "view",
                "type": "function"
            }
        ],
        # Alternative ABI
        [
            {
                "inputs": [{"type": "bytes32", "name": "id"}],
                "name": "pools",
                "outputs": [{"type": "uint128", "name": "liquidity"}],
                "stateMutability": "view", 
                "type": "function"
            }
        ]
    ]
    
    for i, abi in enumerate(abis_to_test):
        try:
            print(f"   Testing ABI {i+1}...")
            contract = w3.eth.contract(address=pool_manager_address, abi=abi)
            
            # Test with a dummy pool ID
            dummy_pool_id = "0x" + "0" * 64
            
            # Try calling the function
            if 'getSlot0' in str(abi):
                result = contract.functions.getSlot0(bytes.fromhex(dummy_pool_id[2:])).call()
                print(f"     ✅ getSlot0 callable: {result}")
            elif 'pools' in str(abi):
                result = contract.functions.pools(bytes.fromhex(dummy_pool_id[2:])).call()
                print(f"     ✅ pools callable: {result}")
                
        except Exception as e:
            print(f"     ❌ ABI {i+1} failed: {e}")
    
    print("\n" + "="*80)

if __name__ == "__main__":
    test_v4_pool_existence()