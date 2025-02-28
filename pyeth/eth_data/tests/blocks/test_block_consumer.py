import asyncio
import json
import os
import aio_pika
import sys
from datetime import datetime


async def main():
    """
    Simple test to verify we can receive at least one block.
    Exits after receiving the first block message.
    """
    # Get RabbitMQ configuration
    rabbitmq_url = os.getenv("RABBITMQ_URL", "amqp://guest:guest@localhost/")
    
    # Event to signal when we've received a message
    received_event = asyncio.Event()
    
    async def process_message(message):
        async with message.process():
            data = json.loads(message.body.decode())
            timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            
            if isinstance(data, list):
                # Handle list format
                if data:
                    block_number = data[0].get('block_number', 'unknown') if isinstance(data[0], dict) else 'unknown'
                    print(f"[{timestamp}] ✅ TEST PASSED: Successfully received block {block_number} data with {len(data)} transactions")
                else:
                    print(f"[{timestamp}] ✅ TEST PASSED: Successfully received block data (empty list)")
            elif isinstance(data, dict):
                # Handle dict format
                block_number = data.get('number', 'unknown')
                tx_count = len(data.get('transactions', []))
                print(f"[{timestamp}] ✅ TEST PASSED: Successfully received block {block_number} with {tx_count} transactions")
            else:
                print(f"[{timestamp}] ✅ TEST PASSED: Successfully received block data of type {type(data)}")
            
            # Signal that we received a message
            received_event.set()
    
    # Connection to RabbitMQ
    connection = None
    try:
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        print(f"[{timestamp}] Connecting to RabbitMQ at {rabbitmq_url}...")
        
        connection = await aio_pika.connect_robust(rabbitmq_url)
        channel = await connection.channel()
        
        # Declare exchange
        exchange = await channel.declare_exchange(
            "blocks_exchange",
            aio_pika.ExchangeType.FANOUT,
            durable=True
        )
        
        # Create temporary queue for testing
        queue = await channel.declare_queue(
            "test_block_consumer",
            durable=True,
            arguments={
                'x-max-length': 100,
                'x-overflow': 'drop-head',
                'x-message-ttl': 3600000,
            }
        )
        
        # Bind queue to exchange
        await queue.bind(exchange, routing_key="processed_blocks")
        
        # Set up consumer
        consumer_tag = await queue.consume(process_message)
        
        print(f"[{timestamp}] Waiting for a block message to verify RabbitMQ setup...")
        print(f"[{timestamp}] Test will exit after receiving the first block")
        
        # Wait for the signal that we received a message
        try:
            # Wait for 60 seconds max
            await asyncio.wait_for(received_event.wait(), 60)
            print(f"[{timestamp}] Test complete - shutting down")
            return 0
        except asyncio.TimeoutError:
            print(f"[{timestamp}] Test timed out after 60 seconds - no blocks received")
            return 1
            
    except Exception as e:
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        print(f"[{timestamp}] Error: {e}")
        return 1
        
    finally:
        # Clean up resources
        if connection is not None and not connection.is_closed:
            await connection.close()
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        print(f"[{timestamp}] Connection closed")


if __name__ == "__main__":
    try:
        exit_code = asyncio.run(main())
        sys.exit(exit_code)
    except KeyboardInterrupt:
        print(f"[{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}] Test interrupted")
        sys.exit(130)
