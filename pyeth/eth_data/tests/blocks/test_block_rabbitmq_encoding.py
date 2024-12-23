"""
Test Block Encoding

Objective:
---------
Verify and debug serialization issues with specific blocks, particularly block 21442705
which failed in production with integer overflow errors.

Test Flow:
---------
1. Process the specific block using BlockProcessor
2. Attempt to serialize the processed block
3. Verify serialization works with our custom serializer
4. Test edge cases in the data that might cause encoding issues
"""

import asyncio
import pytest
import orjson
from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_block_processor.blockchain.live_block_processor import transaction_serializer


TEST_BLOCK_NUMBER_1 = 21442705
TEST_BLOCK_NUMBER_2 = 21446809 # has taken very long to process (3 seconds at least 10x slower than normal)

@pytest.mark.asyncio
async def test_block_encoding(block_number: int):
    """Test encoding of test block which failed in production"""
    
    # Initialize block processor
    processor = BlockProcessor(
        save_erc20_txn_to_db=False
    )
    
    try:
        # Process the specific block
        processed_block = await processor.process_block(block_number)
        
        # Attempt to encode with different approaches to identify the issue
        try:
            # 1. Try direct JSON encoding (should fail)
            encoded_direct = orjson.dumps(processed_block)
            print("Direct encoding succeeded (unexpected)")
        except Exception as e:
            print(f"Direct encoding failed (expected): {e}")
            
        try:
            # 2. Try with our custom serializer
            encoded_custom = orjson.dumps(
                processed_block,
                default=transaction_serializer,
                option=orjson.OPT_SERIALIZE_NUMPY
            )
            print("Custom serializer encoding succeeded")
            
            # Decode and verify the data
            decoded = orjson.loads(encoded_custom)
            print(f"Successfully encoded and decoded block with {len(processed_block)} transactions")
            
            # Print some stats about the encoded data
            print("\nEncoding Statistics:")
            print(f"Original block transactions: {len(processed_block)}")
            print(f"Encoded size: {len(encoded_custom)} bytes")
            
            # Check for potential problematic values
            for tx in processed_block:
                # Check for large integers
                for transfer in tx.erc20_transfers:
                    amount = int(transfer.amount)
                    if amount > 2**63 - 1:
                        print(f"Large ERC20 transfer amount found: {amount}")
                        
                # Check transaction values
                if tx.value > 2**63 - 1:
                    print(f"Large transaction value found: {tx.value}")
                    
        except Exception as e:
            print(f"Custom serializer encoding failed: {e}")
            # Print the problematic transaction if possible
            if 'tx' in locals():
                print(f"\nProblematic transaction: {tx}")
            raise
            
    except Exception as e:
        print(f"Failed to process block: {e}")
        raise
    finally:
        # Cleanup (if needed)
        pass

if __name__ == "__main__":
    asyncio.run(test_block_encoding(TEST_BLOCK_NUMBER_1))
    asyncio.run(test_block_encoding(TEST_BLOCK_NUMBER_2))
