import asyncio
import signal
import sys

from eth_data.blockchain.live_block_processor import LiveBlockProcessor
from eth_data.utils.logger import get_logger


logger = get_logger(name="live_block_processor")

# Store references to tasks and resources for proper cleanup
_tasks = set()
_processor = None


async def shutdown(signal=None):
    """
    Gracefully shut down all running tasks and clean up resources.
    
    Args:
        signal: The signal that triggered the shutdown (optional)
    """
    if signal:
        logger.info(f"Received exit signal {signal.name}...")
    
    # Cancel all running tasks
    logger.info("Cancelling running tasks...")
    for task in _tasks:
        if not task.done():
            task.cancel()
    
    # Wait for tasks to complete cancellation
    if _tasks:
        await asyncio.gather(*_tasks, return_exceptions=True)
    
    # Clean up processor resources if it exists
    if _processor:
        logger.info("Cleaning up processor resources...")
        await _processor.cleanup()
    
    logger.info("Shutdown complete")


async def main(index_address_txs: bool = True):
    """
    Main entry point for the Ethereum Alert System.
    
    Objective:
    - Initialize and run LiveBlockProcessor and AlertManager concurrently.
    - Ensure both components run continuously and handle their respective tasks.
    - Manage graceful shutdown and resource cleanup.
    """
    global _processor
    
    # Initialize components
    _processor = LiveBlockProcessor(
        index_address_txs=index_address_txs,
        rabbitmq_url="amqp://guest:guest@127.0.0.1/",
        logger=logger
    )
    
    # Register signal handlers for graceful shutdown
    for sig in (signal.SIGTERM, signal.SIGINT):
        loop = asyncio.get_running_loop()
        loop.add_signal_handler(sig, lambda s=sig: asyncio.create_task(shutdown(s)))
    
    try:
        # Start monitoring new blocks (don't use monitor_new_blocks directly - use run)
        block_processor_task = asyncio.create_task(_processor.run())
        _tasks.add(block_processor_task)
        
        # Wait for the task to complete (should only happen on shutdown)
        await block_processor_task
        
    except asyncio.CancelledError:
        logger.info("Main task cancelled")
    except Exception as e:
        logger.error(f"Unexpected error in main process: {e}", exc_info=True)
    finally:
        # Ensure proper cleanup if we haven't already done so
        await shutdown()


if __name__ == "__main__":
    try:
        asyncio.run(main(index_address_txs=True))
    except KeyboardInterrupt:
        # This is a fallback - the signal handler should catch most interrupts
        logger.info("KeyboardInterrupt received. Exiting...")