"""
Mempool Monitoring Test Script

Objective:
---------
1. Test the MempoolProcessor's ability to track pending transactions
2. Monitor state diffs for transactions in real-time
3. Log mempool metrics and transaction samples
4. Export transaction data for analysis
"""

import asyncio
import signal
from web3 import Web3
from eth_block_processor.blockchain.mempool_processor import MempoolProcessor
from eth_token.utils.logger import get_logger


async def main():
    # Set up logging
    logger = get_logger(name="mempool_monitor", log_folder="live")
    logger.info("Starting mempool monitoring")
    
    # Create Web3 instance
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    if not w3.is_connected():
        logger.error("Could not connect to Ethereum node. Please check your connection.")
        return
    
    logger.info(f"Connected to Ethereum node: current block is {w3.eth.block_number}")
    
    # Create mempool processor
    processor = MempoolProcessor(
        w3=w3,
        poll_interval=0.5,  # Poll every half second
        max_transactions=2000,  # Keep up to 2000 transactions
        logger=logger
    )
    
    try:
        # Start monitoring
        await processor.start()
        logger.info("Mempool processor started")
        
        # Keep running until interrupted
        while True:
            try:
                await asyncio.sleep(1)
            except asyncio.CancelledError:
                break
            
    except KeyboardInterrupt:
        logger.info("Keyboard interrupt received")
    except Exception as e:
        logger.error(f"Error in main loop: {e}")
    
    finally:
        # Clean shutdown
        logger.info("Shutting down mempool processor...")
        await processor.stop()
        logger.info("Mempool processor stopped. Exiting.")


def handle_signals():
    """Handle keyboard interrupt in Windows and Unix."""
    if hasattr(signal, 'SIGINT'):
        signal.signal(signal.SIGINT, signal.default_int_handler)


if __name__ == "__main__":
    handle_signals()  # Setup signal handling before starting
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nShutdown complete")

