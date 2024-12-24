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

# Define test blocks as constants
TEST_BLOCK_NUMBER_1 = 21442705
TEST_BLOCK_NUMBER_2 = 21446809  # has taken very long to process

TEST_BLOCK_NUMBERS = [TEST_BLOCK_NUMBER_1,
                        TEST_BLOCK_NUMBER_2]


@pytest.mark.asyncio
async def test_block_encoding():  # Remove the parameter
    """Test encoding of test blocks which failed in production"""
    
    # Test both block numbers
    for block_number in TEST_BLOCK_NUMBERS:
        # Initialize block processor
        processor = BlockProcessor(
            save_erc20_txn_to_db=False
        )
        
        try:
            # Process the specific block
            processed_block = await processor.process_block(block_number)
            
            # Attempt to encode with different approaches
            try:
                # 1. Try direct JSON encoding (should fail)
                encoded_direct = orjson.dumps(processed_block)
                print("Direct encoding succeeded (unexpected)")
            except Exception as e:
                print(f"Direct encoding failed (expected): {e}")
                
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
            print(f"\nEncoding Statistics for block {block_number}:")
            print(f"Original block transactions: {len(processed_block)}")
            print(f"Encoded size: {len(encoded_custom)} bytes")
            
        except Exception as e:
            print(f"Failed to process/encode block {block_number}: {e}")
            raise

if __name__ == "__main__":
    asyncio.run(test_block_encoding())
