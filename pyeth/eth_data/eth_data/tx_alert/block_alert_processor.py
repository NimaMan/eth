"""
Block Alert Processor Module

Objective:
- Consume block data (list of transaction dictionaries) from RabbitMQ
- Process transactions using TxnAlertProcessor
- Publish alerts back to RabbitMQ
- Handle priorities and concurrent processing

Flow:
1. Consume blocks (containing a list of DetailedTransactions in a dict) from RabbitMQ blocks_exchange
2. Process transactions concurrently using TxnAlertProcessor
3. Publish generated alerts to alerts_exchange
"""
import time 
import asyncio
from typing import List
from eth_data.tx_alert.txn_alert_processor import TransactionAlertProcessor
from eth_data.tx_processor.data_models.txn_models import ProcessedTransaction
from eth_data.utils.logger import get_logger



class BlockAlertProcessor:
    def __init__(self, logger=None):
        self.logger = logger or get_logger(name="alert_processor")
        self.txn_alert_processor = TransactionAlertProcessor(logger=self.logger)

    async def process_block_transactions(self, txn_list: List[ProcessedTransaction]):
        """Process list of transaction dictionaries concurrently"""
        try:
            start_time = time.time()
            block_number = txn_list[0].block_number if txn_list else None
            
            # Process transactions concurrently
            txn_tasks = [
                self.txn_alert_processor.process_transaction(txn)
                for txn in txn_list
            ]
            
            # Wait for all transaction processing to complete
            alerts_nested = await asyncio.gather(*txn_tasks, return_exceptions=True)
            
            # Flatten and filter alerts
            valid_alerts = []
            for alert_list in alerts_nested:
                if isinstance(alert_list, Exception):
                    continue
                if isinstance(alert_list, list):
                    valid_alerts.extend([
                        alert for alert in alert_list 
                        if alert and not isinstance(alert, (Exception, list))
                    ])
            
            self.logger.info(f"Processed block {block_number} in {time.time() - start_time:.2f} seconds with {len(valid_alerts)} alerts")
            return valid_alerts
        
        except Exception as e:
            self.logger.error(f"{__name__}: Error processing transactions: {e}", exc_info=True)
            return []