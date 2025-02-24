"""
Subscriber for processing alert data from RabbitMQ
"""

import json
from typing import Optional, Callable, Dict, List
import aio_pika
from eth_token.subscribers.base_subscriber import BaseSubscriber
from eth_token.utils.logger import get_logger


logger = get_logger(log_folder="tokens_live")


class AlertSubscriber(BaseSubscriber):
    def __init__(
        self,
        rabbitmq_url: str = "amqp://guest:guest@localhost/",
        callback: Optional[Callable] = None
    ):
        super().__init__(
            rabbitmq_url=rabbitmq_url,
            queue_name="token_analyzer_alerts",
            exchange_name="alerts_exchange",
            routing_key="alerts"
        )
        self.callback = callback
        self.alerts_by_token: Dict[str, List[Dict]] = {}

    async def process_message(self, message: aio_pika.Message):
        """Process incoming alert data"""
        try:
            async with message.process():
                alerts = json.loads(message.body.decode())
                
                for alert in alerts:
                    token_address = alert.get("token_address")
                    if token_address:
                        if token_address not in self.alerts_by_token:
                            self.alerts_by_token[token_address] = []
                        self.alerts_by_token[token_address].append(alert)
                        
                        if self.callback:
                            await self.callback(alert)
                            
                logger.debug(f"Processed {len(alerts)} alerts")

        except Exception as e:
            logger.error(f"Error processing alert message: {e}")

    async def start(self):
        """Start consuming messages"""
        await self.connect()
        async with self.queue.iterator() as queue_iter:
            async for message in queue_iter:
                await self.process_message(message)
