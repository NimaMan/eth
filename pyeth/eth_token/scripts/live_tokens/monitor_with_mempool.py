
import asyncio
from eth_token.token_manager.block_token_processor import BlockTokenProcessor
from eth_token.subscribers.mempool_subscriber import     TokenMempoolTracker

from eth_token.utils.logger import get_logger


async def main():
    logger = get_logger(name="main", log_folder="live")
    
    # Create your regular token processor
    block_token_processor = BlockTokenProcessor(logger=logger)
    
    # Wrap it with mempool awareness
    mempool_aware_processor = TokenMempoolTracker(
        token_processor=block_token_processor,
        logger=logger,
        poll_interval=0.5
    )
    
    # Start the enhanced system
    await mempool_aware_processor.start()
    
    try:
        # Keep running
        while True:
            await asyncio.sleep(10)
            
            # Optionally print stats periodically
            stats = mempool_aware_processor.mempool_detector.get_statistics()
            logger.info(f"Mempool monitoring stats: {stats}")
            
    except KeyboardInterrupt:
        logger.info("Shutting down...")
    finally:
        await mempool_aware_processor.stop()

if __name__ == "__main__":
    asyncio.run(main())