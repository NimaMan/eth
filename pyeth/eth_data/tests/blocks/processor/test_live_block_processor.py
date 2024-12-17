import asyncio
import os
from dotenv import load_dotenv

from eth_block_processor.blockchain.live_block_processor import LiveBlockProcessor
from eth_block_processor.utils.logger import get_logger

# Load environment variables
load_dotenv()

logger = get_logger(name="test_block_processor", log_folder="eth_block_processor")

async def main():
    """
    Test script for LiveBlockProcessor.
    
    Objective:
    - Verify that LiveBlockProcessor successfully connects to Ethereum node
    - Confirm that blocks are being processed correctly
    - Validate that processed blocks are being published to RabbitMQ
    """
    # Get configuration from environment variables
    websocket_url = os.getenv("ETH_WEBSOCKET_URL", "ws://127.0.0.1:8546")
    http_url = os.getenv("ETH_HTTP_URL", "http://127.0.0.1:8545")
    rabbitmq_url = os.getenv("RABBITMQ_URL", "amqp://guest:guest@localhost/")

    try:
        # Initialize LiveBlockProcessor
        processor = LiveBlockProcessor(
            websocket_url=websocket_url,
            http_url=http_url,
            rabbitmq_url=rabbitmq_url,
            save_erc20_txns=False  # Set to True if you want to test ERC20 transaction processing
        )

        logger.info("Starting LiveBlockProcessor test...")
        logger.info(f"WebSocket URL: {websocket_url}")
        logger.info(f"HTTP URL: {http_url}")
        logger.info(f"RabbitMQ URL: {rabbitmq_url}")

        # Run the processor
        await processor.run()

    except KeyboardInterrupt:
        logger.info("Test interrupted by user")
    except Exception as e:
        logger.error(f"Test failed with error: {e}")
    finally:
        # Cleanup will be handled by processor.run()
        logger.info("Test complete")

if __name__ == "__main__":
    asyncio.run(main())
