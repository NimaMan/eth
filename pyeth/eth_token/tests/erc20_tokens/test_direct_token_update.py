#!/usr/bin/env python3
"""Test token update with pool events directly"""

import asyncio
import sys
sys.path.append('/home/nima/code/crypto/py/eth_block_processor')
sys.path.append('/home/nima/code/crypto/py/eth_token')

from web3 import Web3
from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_token.erc20_token.erc20_token import ERC20Token

async def test_direct_update():
    """Test updating a token with a block containing pool events"""
    
    # Process a block that we know has pool events
    processor = BlockProcessor(node_url="http://127.0.0.1:8545")
    
    # Use block 22650601 which has a V2 pair creation
    block_number = 22650601
    print(f"Processing block {block_number} which contains a V2 pair creation...")
    
    processed_txns = await processor.process_block(block_number)
    
    # Find the transaction with pool events
    for txn in processed_txns:
        if txn.pair_events:
            print(f"\nFound transaction with {len(txn.pair_events)} pair events")
            print(f"Transaction hash: {txn.hash}")
            
            # Get the tokens involved
            for event in txn.pair_events:
                print(f"\nPair event:")
                print(f"  Pair: {event.pair_address}")
                print(f"  Token0: {event.token0}")
                print(f"  Token1: {event.token1}")
                
                # Create LiveERC20Token instances for the tokens
                # Skip WETH
                if event.token0 == '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2':
                    token_address = event.token1
                else:
                    token_address = event.token0
                
                print(f"\nTesting token update for {token_address}")
                
                # First check if this is the transaction that creates the token
                if txn.contract_address == token_address:
                    print(f"This transaction creates the token!")
                    # Need to process the creation first
                
                # Create token instance
                token = ERC20Token(token_address)
                
                # Convert ProcessedTransaction to dict for update
                txn_dict = {
                    'hash': txn.hash,
                    'block_number': txn.block_number,
                    'block_timestamp': txn.block_timestamp,
                    'txn_index': txn.txn_index,
                    'from_address': txn.from_address,
                    'to_address': txn.to_address,
                    'status': txn.status,
                    'txn_type': txn.txn_type,
                    'contract_address': txn.contract_address,
                    'contract_creation_events': [
                        {
                            'contract_address': evt.contract_address,
                            'contract_type': evt.contract_type,
                            'name': evt.name,
                            'symbol': evt.symbol,
                            'decimals': evt.decimals,
                            'total_supply': evt.total_supply
                        } for evt in txn.contract_creation_events
                    ] if txn.contract_creation_events else [],
                    'pair_events': [
                        {
                            'pair_address': evt.pair_address,
                            'token0': evt.token0,
                            'token1': evt.token1,
                            'log_index': evt.log_index
                        } for evt in txn.pair_events
                    ],
                    'erc20_contracts': list(txn.erc20_contracts),
                    'unique_addresses': list(txn.unique_addresses),
                    'bribe_amount': txn.bribe_amount,
                    'internal_transactions': [],
                    'erc20_transfers': [],
                    'approvals': [],
                    'owner_events': [],
                    'trading_enabled_events': [],
                    'fees': {'txn_fee': 0.0}  # Add missing fees field
                }
                
                print(f"\nUpdating token with transaction containing {len(txn_dict['pair_events'])} pair events")
                
                # Update token
                token.update_from_transaction(txn_dict)
                
                # Check if pool was detected
                print(f"\nToken state after update:")
                print(f"  Has V2 pool: {token.token_data.has_uni_v2_pool}")
                print(f"  Pool addresses: {token.token_data.pool_addresses}")
                print(f"  Pool manager initialized: {token.token_data.pool_manager is not None}")
                
                if token.token_data.pool_manager:
                    print(f"  Pools in manager: {len(token.token_data.pool_manager.get_all_pools())}")
                
                break  # Just test the first one
            break
    
    await processor.close()

if __name__ == "__main__":
    asyncio.run(test_direct_update())