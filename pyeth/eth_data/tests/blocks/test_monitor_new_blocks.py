import asyncio
from eth_data.blockchain.live_block_processor import LiveBlockProcessor
from eth_data.utils.logger import get_logger
import pytest

# Initialize the logger
logger = get_logger(name="test_live_block_processor", log_folder="tests")


@pytest.mark.asyncio
async def test_live_block_processor():
    # Initialize LiveBlockProcessor with your node and RabbitMQ URLs
    live_block_processor = LiveBlockProcessor(
        websocket_url="ws://127.0.0.1:8546",
        http_url="http://127.0.0.1:8545",
    )
    
    try:
        # Setup RabbitMQ connection
        await live_block_processor.setup_rabbitmq()
        logger.info("RabbitMQ setup completed.")
        
        # Get the latest block number
        latest_block_number = await live_block_processor.w3.eth.block_number
        logger.info(f"Latest block number: {latest_block_number}")
        
        # Process the latest block
        processed_block = await live_block_processor.process_latest_block(block_number=latest_block_number)
        
        if processed_block:
            logger.info(f"Successfully processed block {processed_block['number']}")
            
            # Publish the block to RabbitMQ
            await live_block_processor.publish_block(processed_block)
            logger.info(f"Block {processed_block['number']} published to RabbitMQ.")
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