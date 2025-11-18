import asyncio
import pytest

from eth_data.blockchain.live_block_processor import LiveBlockProcessor
from eth_data.utils.logger import get_logger

# Initialize the logger
logger = get_logger(name="test_live_block_processor", log_folder="tests")


class _FakePublisher:
    def __init__(self):
        self.published = []

    async def publish(self, channel, payload):
        self.published.append((channel, payload))


@pytest.mark.asyncio
@pytest.mark.skip(reason="Integration test that requires local node access")
async def test_live_block_processor():
    # Initialize LiveBlockProcessor with your node URLs
    live_block_processor = LiveBlockProcessor(
        websocket_url="ws://127.0.0.1:8546",
        http_url="http://127.0.0.1:8545",
    )
    live_block_processor._block_signal_publisher = _FakePublisher()
    
    try:
        # Get the latest block number
        latest_block_number = await live_block_processor.w3.eth.block_number
        logger.info(f"Latest block number: {latest_block_number}")
        
        # Process the latest block
        processed_block = await live_block_processor.block_processor.process_block(
            block_number=latest_block_number
        )
        
        if processed_block:
            logger.info(f"Successfully processed block {processed_block['block_number']}")
            
            # Publish the block notification to Redis (mocked)
            await live_block_processor.publish_block_notification(latest_block_number)
            assert live_block_processor._block_signal_publisher.published
        else:
            logger.error(f"Failed to process block {latest_block_number}")
    
    except Exception as e:
        logger.error(f"An error occurred: {e}")
    
    finally:
        # Cleanup connections
        await live_block_processor.cleanup()
        logger.info("Cleanup completed.")

if __name__ == "__main__":
    asyncio.run(test_live_block_processor())
