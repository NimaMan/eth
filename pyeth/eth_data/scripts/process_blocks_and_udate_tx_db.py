import asyncio
import argparse
from tqdm import tqdm
from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_block_processor.utils.logger import get_logger


async def process_blocks(
    node_url: str,
    num_blocks: int = 1000000,
    save_txn_to_db: bool = True,
    start_block: int = None,
    end_block: int = None
):
    """
    Process Ethereum blocks sequentially, skipping already processed blocks.
    
    Args:
        node_url: URL of the Ethereum node
        num_blocks: Number of blocks to process
        save_txn_to_db: Whether to save transactions to the database
        start_block: Starting block number (optional)
        end_block: Ending block number (optional)
    """
    logger = get_logger(name="db_processor")
    
    # Initialize processor with database saving enabled
    processor = BlockProcessor(
        node_url=node_url,
        save_txn_to_db=save_txn_to_db,
        logger=logger
    )
    
    try:
        # Determine block range
        if end_block is None:
            # Get latest block number
            latest_block = await processor.block_fetcher.w3.eth.get_block_number()
            end_block = latest_block
        
        if start_block is None:
            # Calculate start block based on the number of blocks to process
            start_block = max(0, end_block - num_blocks + 1)
        
        logger.info(f"Block range: {start_block} to {end_block}")
        
        # Get already processed blocks from the database
        if save_txn_to_db:
            processed_blocks = processor.transaction_saver.get_processed_blocks()
            logger.info(f"Found {len(processed_blocks)} already processed blocks in the database")
        else:
            processed_blocks = set()
        
        for block_number in tqdm(range(end_block, start_block - 1, -1)):
            # Skip if already processed
            if block_number in processed_blocks:
                logger.debug(f"Skipping already processed block {block_number}")
                continue
            
            try:
                # Process the block
                result = await processor.process_block(block_number)
                        
            except Exception as e:
                logger.error(f"Error processing block {block_number}: {str(e)}")
        
    except Exception as e:
        logger.error(f"Error in process_blocks: {str(e)}")
        raise


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Process Ethereum blocks and save transactions to database")
    parser.add_argument("--node-url", default="http://127.0.0.1:8545", help="Ethereum node URL")
    parser.add_argument("--blocks", type=int, default=1000000, help="Total number of blocks to process")
    parser.add_argument("--start-block", type=int, help="Starting block number (optional)")
    parser.add_argument("--end-block", type=int, help="Ending block number (optional)")
    parser.add_argument("--no-save", action="store_true", help="Don't save transactions to database")
    
    args = parser.parse_args()
    
    try:
        asyncio.run(process_blocks(
            node_url=args.node_url,
            num_blocks=args.blocks,
            save_txn_to_db=not args.no_save,
            start_block=args.start_block,
            end_block=args.end_block
        ))
    except KeyboardInterrupt:
        print("\nProcess interrupted by user")
    except Exception as e:
        print(f"Error: {str(e)}")
