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
from eth_data.blockchain.block_processor import BlockProcessor
from eth_data.blockchain.live_block_processor import transaction_serializer

# Define test blocks as constants
TEST_BLOCK_NUMBER_1 = 22003306
TEST_BLOCK_NUMBER_2 = 22003328  
TEST_BLOCK_NUMBER_3 = 22003336

TEST_BLOCK_NUMBERS = [
    TEST_BLOCK_NUMBER_1,
    TEST_BLOCK_NUMBER_2,
    TEST_BLOCK_NUMBER_3
]


@pytest.mark.asyncio
async def test_block_encoding():
    """Test encoding of test blocks which failed in production"""
    
    for block_number in TEST_BLOCK_NUMBERS:
        processor = BlockProcessor()
        
        try:
            processed_block_result = await processor.process_block(block_number)
            transactions = processed_block_result.transactions
            print(f"\nProcessing block {block_number} with {len(transactions)} transactions")
            
            failed_txs = []
            for idx, tx in enumerate(transactions):
                try:
                    # Try to encode each transaction individually
                    encoded_tx = orjson.dumps(
                        tx,
                        default=transaction_serializer,
                        option=orjson.OPT_SERIALIZE_NUMPY
                    )
                except Exception as e:
                    failed_txs.append({
                        'index': idx,
                        'hash': tx.hash if hasattr(tx, 'hash') else 'N/A',
                        'error': str(e),
                        'problematic_fields': {}
                    })
                    
                    # Check each field for encoding issues
                    for attr_name, attr_value in tx.__dict__.items():
                        try:
                            orjson.dumps({attr_name: attr_value}, default=transaction_serializer)
                        except Exception as field_error:
                            failed_txs[-1]['problematic_fields'][attr_name] = {
                                'type': str(type(attr_value)),
                                'value': str(attr_value),
                                'error': str(field_error)
                            }
            
            # If any transactions failed to encode, print details and fail the test
            if failed_txs:
                print("\nEncoding failures detected:")
                for failed_tx in failed_txs:
                    print(f"\nTransaction {failed_tx['index']} (hash: {failed_tx['hash']}):")
                    print(f"Error: {failed_tx['error']}")
                    print("\nProblematic fields:")
                    for field_name, field_info in failed_tx['problematic_fields'].items():
                        print(f"\n  {field_name}:")
                        print(f"    Type: {field_info['type']}")
                        print(f"    Value: {field_info['value']}")
                        print(f"    Error: {field_info['error']}")
                
                raise ValueError(f"Failed to encode {len(failed_txs)} transactions in block {block_number}")
            
            # Try encoding the full block
            try:
                encoded_custom = orjson.dumps(
                    transactions,
                    default=transaction_serializer,
                    option=orjson.OPT_SERIALIZE_NUMPY
                )
                print(f"Successfully encoded block {block_number}")
                print(f"Encoded size: {len(encoded_custom)} bytes")
                
            except Exception as e:
                print(f"\nFailed to encode full block {block_number}:")
                print(f"Error: {str(e)}")
                raise
            
        except Exception as e:
            print(f"Failed to process/encode block {block_number}")
            print(f"Error: {str(e)}")
            raise

if __name__ == "__main__":
    asyncio.run(test_block_encoding())
