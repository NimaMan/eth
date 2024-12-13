import asyncio
from ethblockprocessor.blockchain.block_processor import BlockProcessor
from general_utils.logging.logger import get_logger


logger = get_logger(name=__name__, log_folder="eth_block_processor")


async def main():
    block_processor = BlockProcessor(
        save_alert_db=True,
        save_erc20_txn_to_db=True
    )

    # Process the latest block
    logger.info("Processing the latest block...")
    latest_block = await block_processor.block_fetcher.fetch_latest_block_number()
    logger.info(f"Processed latest block: {latest_block}")

    N = 99
    logger.info(f"Processing the last {N} blocks...")
    start_block = latest_block - N
    processed_blocks = await block_processor.process_block_range(start_block, latest_block)
    logger.info(f"Processed {len(processed_blocks)} blocks from {start_block} to {latest_block}")

    # Log any errors or exceptions
    for block_number, result in processed_blocks.items():
        if isinstance(result, Exception):
            logger.error(f"Error processing block {block_number}: {str(result)}")

if __name__ == "__main__":
    asyncio.run(main())
