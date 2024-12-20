import asyncio

from eth_block_processor.blockchain.live_block_processor import LiveBlockProcessor
from eth_block_processor.utils.logger import get_logger


logger = get_logger(name="block_processor", log_folder="eth_block_processor")


async def main():
    """
    Main entry point for the Ethereum Alert System.
    
    Objective:
    - Initialize and run LiveBlockProcessor and AlertManager concurrently.
    - Ensure both components run continuously and handle their respective tasks.
    - Manage graceful shutdown and resource cleanup.
    """
    # Initialize components
    live_processor = LiveBlockProcessor()
    
    # Start monitoring new blocks
    block_processor_task = asyncio.create_task(
        live_processor.monitor_new_blocks(),
        name="LiveBlockProcessor"
    )
    try:
        # Run both tasks concurrently
        await asyncio.gather(block_processor_task)
    except asyncio.CancelledError:
        logger.info("Main tasks have been cancelled. Initiating shutdown.")
    finally:
        # Ensure proper cleanup
        if hasattr(live_processor.w3, 'provider'):
            await live_processor.w3.provider.disconnect()
        logger.info("Shutdown complete.")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        logger.info("KeyboardInterrupt received. Exiting...")