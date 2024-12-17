import asyncio
import json
from typing import Any, List

import aio_pika

from eth_block_processor.alert.alert_queue import processed_alert_queue
from eth_block_processor.alert.alert_db import AlertDB
from eth_block_processor.utils.logger import get_logger


logger = get_logger("alert_exporter", log_folder="alert_exporter")


class AlertExporter:
    def __init__(self, rabbitmq_url: str = "amqp://guest:guest@localhost/", batch_size: int = 50, export_interval: float = 2.0):
        self.rabbitmq_url = rabbitmq_url
        self.batch_size = batch_size
        self.export_interval = export_interval
        self._batch: List[Any] = []
        self._lock = asyncio.Lock()

    async def consume_alerts(self):
        """Consume processed alerts from RabbitMQ and persist them to the database."""
        connection = await aio_pika.connect_robust(self.rabbitmq_url)
        async with connection:
            channel = await connection.channel()
            exchange = await channel.declare_exchange("alerts_exchange", aio_pika.ExchangeType.FANOUT)
            queue = await channel.declare_queue("", exclusive=True)
            await queue.bind(exchange)

            logger.info("AlertExporter is consuming from alerts_exchange")

            async for message in queue:
                async with message.process():
                    try:
                        alert = json.loads(message.body.decode())
                        async with self._lock:
                            self._batch.append(alert)
                            if len(self._batch) >= self.batch_size:
                                await self._export_batch()
                    except Exception as e:
                        logger.error(f"Error processing alert message: {e}")

    async def _export_batch(self):
        """Persist a batch of alerts to the database."""
        if not self._batch:
            return
        batch_to_export = self._batch.copy()
        self._batch.clear()

        try:
            async with AlertDB() as db:
                await db.add_alerts(batch_to_export)
            logger.info(f"Exported {len(batch_to_export)} alerts to the database.")
        except Exception as e:
            logger.error(f"Error exporting alerts to the database: {e}")
            # Optionally, re-queue alerts or implement a retry mechanism

    async def periodic_export(self):
        """Periodically export remaining alerts based on export_interval."""
        while True:
            await asyncio.sleep(self.export_interval)
            async with self._lock:
                await self._export_batch()

    async def run(self):
        """Run the AlertExporter components concurrently."""
        consumer_task = asyncio.create_task(self.consume_alerts(), name="AlertExporterConsumer")
        periodic_task = asyncio.create_task(self.periodic_export(), name="AlertExporterPeriodic")
        await asyncio.gather(consumer_task, periodic_task)

