import asyncio
import os
import logging
from datetime import datetime
from ethblockprocessor.blockchain.block_processor import BlockProcessor

# Set up logging
log_dir = "/home/nima/code/crypto/logs"
os.makedirs(log_dir, exist_ok=True)
log_file = os.path.join(log_dir, f"block_processor_{datetime.now().strftime('%Y%m%d_%H%M%S')}.log")
logging.basicConfig(filename=log_file, level=logging.INFO,
                    format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

async def main():
    block_processor = BlockProcessor(
        save_alert_db=True,
        save_erc20_txn_to_db=True
    )

    # Process the latest block
    logger.info("Processing the latest block...")
    latest_block = await block_processor.process_latest_block()
    logger.info(f"Processed latest block: {latest_block}")

    # Process the last 10,000 blocks
    logger.info("Processing the last 10,000 blocks...")
    start_block = latest_block - 19999
    processed_blocks = await block_processor.process_block_range(start_block, latest_block)
    logger.info(f"Processed {len(processed_blocks)} blocks from {start_block} to {latest_block}")

    # Log any errors or exceptions
    for block_number, result in processed_blocks.items():
        if isinstance(result, Exception):
            logger.error(f"Error processing block {block_number}: {str(result)}")

if __name__ == "__main__":
    asyncio.run(main())
