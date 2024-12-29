"""
Base subscriber class for handling RabbitMQ connections and message processing
"""

import aio_pika
from typing import Optional, Callable, Any


class BaseSubscriber:
    def __init__(
        self,
        rabbitmq_url: str = "amqp://guest:guest@localhost/",
        queue_name: str = "",
        exchange_name: str = "",
        routing_key: str = "",
    ):
        self.rabbitmq_url = rabbitmq_url
        self.queue_name = queue_name
        self.exchange_name = exchange_name
        self.routing_key = routing_key
        
        self.connection: Optional[aio_pika.Connection] = None
        self.channel: Optional[aio_pika.Channel] = None
        self.queue: Optional[aio_pika.Queue] = None
        self.exchange: Optional[aio_pika.Exchange] = None
        
    async def connect(self):
        """Establish connection to RabbitMQ"""
        try:
            self.connection = await aio_pika.connect_robust(self.rabbitmq_url)
            self.channel = await self.connection.channel()
            self.exchange = await self.channel.declare_exchange(
                self.exchange_name,
                aio_pika.ExchangeType.FANOUT,
                durable=True
            )
            self.queue = await self.channel.declare_queue(
                self.queue_name,
                durable=True
            )
            await self.queue.bind(self.exchange, routing_key=self.routing_key)
            self.logger.info(f"Connected to RabbitMQ exchange: {self.exchange_name}")
        except Exception as e:
            self.logger.error(f"Failed to connect to RabbitMQ: {e}")
            raise

    async def disconnect(self):
        """Close RabbitMQ connection"""
        if self.connection and not self.connection.is_closed:
            await self.connection.close()
            self.logger.info("Disconnected from RabbitMQ")

    async def process_message(self, message: aio_pika.Message):
        """Override this method in derived classes"""
        raise NotImplementedError
