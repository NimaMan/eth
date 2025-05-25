#!/usr/bin/env python3
"""
Simple test script to verify broadcast messaging works

This script creates a simple consumer that connects to the blocks_exchange
and verifies that multiple consumers can receive the same blocks.

Usage:
    python test_broadcast_simple.py [consumer_id]

Run multiple instances to test broadcast functionality.
"""

import asyncio
import sys
import signal
import os
import uuid
import orjson
import aio_pika
from datetime import datetime


class SimpleBroadcastConsumer:
    def __init__(self, consumer_id: str):
        self.consumer_id = consumer_id
        self.rabbitmq_url = "amqp://guest:guest@127.0.0.1/"
        self.connection = None
        self.channel = None
        self.queue = None
        self.exchange = None
        self._processing = False
        
        # Create unique queue name for this consumer instance
        self.queue_name = f"test_consumer_{consumer_id}_{uuid.uuid4().hex[:8]}"
        self.exchange_name = "blocks_exchange"
        
    async def connect(self):
        """Establish connection to RabbitMQ with exclusive queue for broadcast"""
        try:
            print(f"[{self.consumer_id}] Connecting to RabbitMQ...")
            self.connection = await aio_pika.connect_robust(self.rabbitmq_url)
            self.channel = await self.connection.channel()
            
            # Connect to the existing blocks exchange
            self.exchange = await self.channel.declare_exchange(
                self.exchange_name,
                aio_pika.ExchangeType.FANOUT,
                durable=True
            )
            
            # Create exclusive, auto-delete queue that keeps only latest block
            self.queue = await self.channel.declare_queue(
                self.queue_name,
                exclusive=True,      # Only this connection can access
                auto_delete=True,    # Deleted when connection closes
                arguments={
                    'x-max-length': 1,           # Keep only 1 message
                    'x-overflow': 'drop-head'    # Drop oldest when new arrives
                }
            )
            
            # Bind to the exchange with empty routing key (FANOUT ignores routing keys)
            await self.queue.bind(self.exchange, routing_key="")
            
            print(f"[{self.consumer_id}] Connected with queue: {self.queue_name}")
            return True
            
        except Exception as e:
            print(f"[{self.consumer_id}] Failed to connect: {e}")
            return False
    
    async def disconnect(self):
        """Close RabbitMQ connection"""
        try:
            if self.connection and not self.connection.is_closed:
                await self.connection.close()
                print(f"[{self.consumer_id}] Disconnected from RabbitMQ")
        except Exception as e:
            print(f"[{self.consumer_id}] Error during disconnect: {e}")
    
    async def process_message(self, message: aio_pika.Message):
        """Process received block message"""
        try:
            async with message.process():
                # Parse the block data
                block_data = orjson.loads(message.body.decode())
                
                # Extract block number from the data structure
                if isinstance(block_data, list) and len(block_data) > 0:
                    block_info = block_data[0]
                    block_number = block_info.get("block_number", "unknown")
                else:
                    block_number = "unknown"
                
                timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
                print(f"[{timestamp}] Consumer {self.consumer_id} received block {block_number}")
                
                # Simulate some processing
                await asyncio.sleep(0.05)
                
        except Exception as e:
            print(f"[{self.consumer_id}] Error processing message: {e}")
    
    async def start(self):
        """Start consuming messages"""
        self._processing = True
        
        if not await self.connect():
            return
        
        try:
            print(f"[{self.consumer_id}] Starting to consume messages...")
            
            # Start consuming messages
            async with self.queue.iterator() as queue_iter:
                async for message in queue_iter:
                    if not self._processing:
                        break
                    await self.process_message(message)
                    
        except Exception as e:
            print(f"[{self.consumer_id}] Error during consumption: {e}")
        finally:
            await self.disconnect()
    
    async def stop(self):
        """Stop consuming messages"""
        print(f"[{self.consumer_id}] Stopping...")
        self._processing = False


async def main():
    """Main function"""
    # Get consumer ID from command line or use process ID
    consumer_id = sys.argv[1] if len(sys.argv) > 1 else f"consumer_{os.getpid()}"
    
    print(f"Starting broadcast test consumer: {consumer_id}")
    print("This consumer will receive blocks published to 'blocks_exchange'")
    print("Run multiple instances to test broadcast functionality")
    print("Press Ctrl+C to stop\n")
    
    consumer = SimpleBroadcastConsumer(consumer_id)
    
    # Setup signal handlers for graceful shutdown
    def signal_handler():
        print(f"\nShutdown signal received for {consumer_id}")
        asyncio.create_task(consumer.stop())
    
    # Register signal handlers
    for sig in (signal.SIGTERM, signal.SIGINT):
        loop = asyncio.get_running_loop()
        loop.add_signal_handler(sig, signal_handler)
    
    try:
        await consumer.start()
    except KeyboardInterrupt:
        print(f"\nKeyboardInterrupt for {consumer_id}")
    finally:
        await consumer.stop()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nExiting...")
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1) 