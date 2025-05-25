#!/usr/bin/env python3
"""
Quick test to verify multiple consumers receive the same blocks

This script runs multiple consumers in the same process to quickly verify
that the broadcast messaging is working correctly.
"""

import asyncio
import uuid
import orjson
import aio_pika
from datetime import datetime


class QuickTestConsumer:
    def __init__(self, consumer_id: str):
        self.consumer_id = consumer_id
        self.rabbitmq_url = "amqp://guest:guest@127.0.0.1/"
        self.connection = None
        self.channel = None
        self.queue = None
        self.exchange = None
        self._processing = False
        self.blocks_received = []
        
        # Create unique queue name
        self.queue_name = f"quick_test_{consumer_id}_{uuid.uuid4().hex[:6]}"
        
    async def connect(self):
        """Connect to RabbitMQ"""
        try:
            self.connection = await aio_pika.connect_robust(self.rabbitmq_url)
            self.channel = await self.connection.channel()
            
            # Connect to blocks exchange
            self.exchange = await self.channel.declare_exchange(
                "blocks_exchange",
                aio_pika.ExchangeType.FANOUT,
                durable=True
            )
            
            # Create exclusive queue
            self.queue = await self.channel.declare_queue(
                self.queue_name,
                exclusive=True,
                auto_delete=True,
                arguments={
                    'x-max-length': 1,
                    'x-overflow': 'drop-head'
                }
            )
            
            await self.queue.bind(self.exchange, routing_key="")
            print(f"Consumer {self.consumer_id} connected")
            return True
            
        except Exception as e:
            print(f"Consumer {self.consumer_id} failed to connect: {e}")
            return False
    
    async def process_message(self, message: aio_pika.Message):
        """Process received message"""
        try:
            async with message.process():
                block_data = orjson.loads(message.body.decode())
                
                if isinstance(block_data, list) and len(block_data) > 0:
                    block_number = block_data[0].get("block_number", "unknown")
                else:
                    block_number = "unknown"
                
                self.blocks_received.append(block_number)
                timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
                print(f"[{timestamp}] Consumer {self.consumer_id} received block {block_number}")
                
        except Exception as e:
            print(f"Consumer {self.consumer_id} error: {e}")
    
    async def start_consuming(self, duration=15):
        """Start consuming for a limited duration"""
        if not await self.connect():
            return
        
        self._processing = True
        print(f"Consumer {self.consumer_id} starting to consume for {duration} seconds...")
        
        try:
            # Use asyncio.wait_for to limit consumption time
            await asyncio.wait_for(self._consume_loop(), timeout=duration)
        except asyncio.TimeoutError:
            print(f"Consumer {self.consumer_id} finished consuming (timeout)")
        except Exception as e:
            print(f"Consumer {self.consumer_id} error during consumption: {e}")
        finally:
            self._processing = False
            if self.connection and not self.connection.is_closed:
                await self.connection.close()
    
    async def _consume_loop(self):
        """Internal consumption loop"""
        async with self.queue.iterator() as queue_iter:
            async for message in queue_iter:
                if not self._processing:
                    break
                await self.process_message(message)


async def main():
    """Run multiple consumers simultaneously"""
    print("Quick broadcast test - Running 3 consumers for 15 seconds")
    print("All consumers should receive the same block numbers\n")
    
    # Create 3 consumers
    consumers = [
        QuickTestConsumer("A"),
        QuickTestConsumer("B"), 
        QuickTestConsumer("C")
    ]
    
    # Start all consumers concurrently
    tasks = [consumer.start_consuming(duration=15) for consumer in consumers]
    
    try:
        await asyncio.gather(*tasks)
    except Exception as e:
        print(f"Error during test: {e}")
    
    # Print results
    print("\n" + "="*50)
    print("TEST RESULTS:")
    print("="*50)
    
    for consumer in consumers:
        print(f"Consumer {consumer.consumer_id}: {len(consumer.blocks_received)} blocks received")
        if consumer.blocks_received:
            print(f"  Blocks: {consumer.blocks_received}")
    
    # Check if all consumers received the same blocks
    if len(consumers) > 1:
        all_blocks = [set(c.blocks_received) for c in consumers]
        if all(blocks == all_blocks[0] for blocks in all_blocks):
            print("\n✅ SUCCESS: All consumers received the same blocks!")
        else:
            print("\n❌ ISSUE: Consumers received different blocks")
            for i, consumer in enumerate(consumers):
                print(f"  Consumer {consumer.consumer_id}: {consumer.blocks_received}")
    
    print("="*50)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nTest interrupted")
    except Exception as e:
        print(f"Test failed: {e}") 