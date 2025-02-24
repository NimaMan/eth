"""
Test script for BlockLiveTokenProcessor

Objective:
---------
Test token processing functionality with specific blocks
without requiring RabbitMQ subscription
"""

from dataclasses import asdict
import asyncio
from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_token.token_manager.live_block_token_processor import BlockLiveTokenProcessor
from eth_token.utils.logger import get_logger


logger = get_logger(name='test_block_live_token_processor', log_folder='tests')


TEST_BLOCKS = [21423372, #  Regular contract creation
               21423691] # ERC-20 contract creation


async def test_token_processing():
    # Initialize processors
    block_processor = BlockProcessor(logger=logger)
    live_token_processor = BlockLiveTokenProcessor(logger=logger)
    
    # Test blocks
    test_blocks = TEST_BLOCKS
    
    for block_number in test_blocks:
        print(f"\nProcessing block {block_number}")
        
        # Get processed block data
        block_data = await block_processor.process_block(block_number=block_number)
        block_data = [asdict(txn) for txn in block_data]
        # Process the block with token processor
        await live_token_processor.process_block(block_data)
        
    # Print statistics
    print(f"Tokens in cache: {len(live_token_processor.live_tokens_cache)}")
    print("New tokens created:")
    for addr, block in live_token_processor.token_first_seen.items():
        token = live_token_processor.live_tokens_cache[addr]
        print(f"- Address: {addr}")
        print(f"  First seen: Block {block}")
        print(f"  Name: {token.name}")
        print(f"  Symbol: {token.symbol}")
    

if __name__ == "__main__":
    asyncio.run(test_token_processing()) 