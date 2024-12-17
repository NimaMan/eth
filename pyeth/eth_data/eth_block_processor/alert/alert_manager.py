import asyncio
import json
from typing import List, Any, Dict, Type
from web3.types import BlockData
import aio_pika

from eth_block_processor.alert.base_alert_class import BaseAlert, AlertPriority
from eth_block_processor.alert.trading_enabled_alert import TradingEnabledAlert
from eth_block_processor.alert.bribe_alert import BribeAlert
from eth_block_processor.alert.contract_creation_alert import ContractCreationAlert
from eth_block_processor.alert.user_involved_alert import GreyAddressAlert, OrcaAlert, WhaleAlert
from eth_block_processor.utils.logger import get_logger


logger = get_logger("alert_manager", log_folder="alert")


ALERT_CLASSES = {
    'trading_enabled': (TradingEnabledAlert, AlertPriority.HIGH),
    'bribe': (BribeAlert, AlertPriority.HIGH),
    'contract_creation': (ContractCreationAlert, AlertPriority.LOW),
    'grey_address': (GreyAddressAlert, AlertPriority.LOW),
    'orca': (OrcaAlert, AlertPriority.MEDIUM),
    'whale': (WhaleAlert, AlertPriority.MEDIUM)
}


class AlertManager:
    """
    AlertManager coordinates asynchronous alert processing based on block data.
    
    Objective:
    - Listen to processed blocks from the shared queue.
    - Process alerts based on defined priorities.
    - Manage alert lifecycle including processing, deduplication, and storage.
    """
    
    def __init__(self, max_workers: int = 4, rabbitmq_url: str = "amqp://guest:guest@localhost/"):
        self.alert_processors: Dict[str, BaseAlert] = {}
        self.rabbitmq_url = rabbitmq_url

        # Initialize alert processors
        for name, (alert_class, priority) in ALERT_CLASSES.items():
            processor = alert_class()
            processor.priority = priority
            self.alert_processors[name] = processor

    async def consume_blocks(self):
        """Consume processed blocks from RabbitMQ and process alerts."""
        connection = await aio_pika.connect_robust(self.rabbitmq_url)
        async with connection:
            channel = await connection.channel()
            exchange = await channel.declare_exchange("blocks_exchange", aio_pika.ExchangeType.FANOUT)
            queue = await channel.declare_queue("", exclusive=True)
            await queue.bind(exchange)

            logger.info("AlertManager is consuming from blocks_exchange")

            async for message in queue:
                async with message.process():
                    try:
                        block = json.loads(message.body.decode())
                        block_number = block['number']
                        logger.debug(f"Received block {block_number} for alert processing")
                        await self.process_block(block)
                    except Exception as e:
                        logger.error(f"Error processing block message: {e}")

    async def process_block(self, block: BlockData) -> None:
        """Process alerts for a new block with priority handling."""
        try:
            # Process high priority alerts immediately
            high_priority_alerts = await self._process_high_priority(block)

            # Enqueue high priority alerts for persistence
            for alert in high_priority_alerts:
                await processed_alert_queue.put(alert)

            # Process medium priority alerts asynchronously
            asyncio.create_task(self._process_medium_priority(block), name=f"medium_priority_block_{block['number']}")

        except Exception as e:
            logger.error(f"Error processing alerts for block {block['number']}: {e}")

    async def _process_high_priority(self, block: BlockData) -> List[Any]:
        """Process high priority alerts."""
        high_priority_processors = [
            processor for processor in self.alert_processors.values()
            if processor.priority == AlertPriority.HIGH
        ]

        results = []
        for processor in high_priority_processors:
            try:
                alerts = await processor.process(block)
                if alerts:
                    results.extend(alerts)
            except Exception as e:
                logger.error(f"Error in {processor.__class__.__name__}: {e}")

        return results

    async def _process_medium_priority(self, block: BlockData) -> None:
        """Process medium priority alerts and enqueue them for persistence."""
        medium_priority_processors = [
            processor for processor in self.alert_processors.values()
            if processor.priority == AlertPriority.MEDIUM
        ]

        for processor in medium_priority_processors:
            try:
                alerts = await processor.process(block)
                if alerts:
                    for alert in alerts:
                        await processed_alert_queue.put(alert)
            except Exception as e:
                logger.error(f"Error in {processor.__class__.__name__}: {e}")
