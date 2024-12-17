import asyncio
import json
from dotenv import load_dotenv
import os
import aio_pika

from eth_block_processor.utils.logger import get_logger

# Load environment variables
load_dotenv()

logger = get_logger(name="test_block_consumer", log_folder="eth_block_processor")

async def process_message(message: aio_pika.IncomingMessage):
    """Process received block message."""
    async with message.process():
        try:
            block = json.loads(message.body.decode())
            logger.info(
                f"Received block {block['number']} with "
                f"{len(block['transactions'])} transactions"
            )
        except Exception as e:
            logger.error(f"Error processing message: {e}")

async def main():
    """
    Test consumer for processed blocks.
    
    Objective:
    - Connect to RabbitMQ and consume messages from blocks_exchange
    - Verify that blocks are being received correctly
    - Log block information for monitoring
    """
    # Get RabbitMQ configuration
    rabbitmq_url = os.getenv("RABBITMQ_URL", "amqp://guest:guest@localhost/")

    try:
        # Connect to RabbitMQ
        connection = await aio_pika.connect_robust(rabbitmq_url)
        async with connection:
            # Create channel
            channel = await connection.channel()
            
            # Declare exchange
            exchange = await channel.declare_exchange(
                "blocks_exchange",
                aio_pika.ExchangeType.FANOUT,
                durable=True
            )
            
            # Declare queue
            queue = await channel.declare_queue("", exclusive=True)
            
            # Bind queue to exchange
            await queue.bind(exchange)
            
            logger.info(
                f"Started consuming from blocks_exchange on {rabbitmq_url}"
            )
            
            # Start consuming messages
            async with queue.iterator() as queue_iter:
                async for message in queue_iter:
                    await process_message(message)

    except Exception as e:
        logger.error(f"Consumer failed with error: {e}")

if __name__ == "__main__":
    asyncio.run(main())
