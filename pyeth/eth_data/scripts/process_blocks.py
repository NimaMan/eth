"""
Script to process and save the most recent 1000 Ethereum blocks.

Algorithmic Description:
-----------------------
1. Connect to Ethereum node and get the latest block number
2. Calculate the start block (latest - 1000)
3. Initialize BlockProcessor with database saving enabled
4. Process the block range and save transactions to database
5. Log progress and any errors encountered
"""

import asyncio
from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_block_processor.utils.logger import get_logger


async def process_recent_blocks(
    node_url: str = "http://127.0.0.1:8545",
    num_blocks: int = 1000,
    save_txn_to_db: bool = False
):
    """
    Process and save the most recent blocks.
    
    Args:
        node_url: URL of the Ethereum node
        num_blocks: Number of recent blocks to process
    """
    logger = get_logger(name="db_processor")
    
    # Initialize processor with database saving enabled
    processor = BlockProcessor(
        node_url=node_url,
        save_txn_to_db=save_txn_to_db,
        logger=logger
    )
    
    try:
        # Get latest block number using the async block fetcher
        latest_block = await processor.block_fetcher.w3.eth.get_block_number()
        start_block = max(0, latest_block - num_blocks + 1)  # Ensure we don't go below 0
        
        logger.info(f"Processing blocks from {start_block} to {latest_block}")
        
        # Process the block range
        results = await processor.process_block_range(start_block, latest_block)
        
        # Log summary
        total_txns = sum(len(block_txns) for block_txns in results.values() if block_txns)
        logger.info(f"Completed processing {len(results)} blocks with {total_txns} transactions")
        
        return results
    
    except Exception as e:
        logger.error(f"Error processing recent blocks: {str(e)}")
        raise

if __name__ == "__main__":
    try:
        asyncio.run(process_recent_blocks(num_blocks=1000000,
                                           save_txn_to_db=True))
    except KeyboardInterrupt:
        print("\nProcess interrupted by user")
    except Exception as e:
        print(f"Error: {str(e)}")
