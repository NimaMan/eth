
import asyncio
from typing import Any, List
from eth_data.tx_alert.bribe_alert import BribeAlert
from eth_data.tx_alert.user_involved_alert import OrcaAlert, WhaleAlert
from eth_data.utils.logger import get_logger

ProcessedTransaction = Any

class TransactionAlertProcessor:
    def __init__(self, logger=None):
        self.logger = logger
        # Initialize all alert processors
        self.alert_processors = {}

    async def process_single_alert(self, alert_type: str, processor, tx: ProcessedTransaction):
        """Process a single alert type asynchronously"""
        try:
            alerts = await processor.process_tx(tx)
            if alerts:
                self.logger.debug(f"Generated {alert_type} alerts for tx {tx.hash}: {len(alerts)}")
            return alerts
        except Exception as e:
            self.logger.error(f"{__name__}: Error processing {alert_type} alert for tx {tx.hash}: {str(e)}")
            return []

    async def process_transaction(self, tx: ProcessedTransaction):
        """
        Process a transaction through all alert processors concurrently
        
        Args:
            tx: Detailed transaction to process
            
        Returns:
            List of alerts generated from all processors
        """
        try:
            # Create tasks for all alert processors
            alert_tasks = [
                self.process_single_alert(alert_type, processor, tx)
                for alert_type, processor in self.alert_processors.items()
            ]
            
            # Execute all alert processing concurrently
            results = await asyncio.gather(*alert_tasks, return_exceptions=True)
            
            # Aggregate alerts, filtering out errors and empty results
            all_alerts = []
            for result in results:
                if isinstance(result, list):
                    all_alerts.extend(result)
                elif isinstance(result, Exception):
                    self.logger.error(f"{__name__}: Alert processing error for tx {tx.hash}: {str(result)}")
            return all_alerts
            
        except Exception as e:
            self.logger.error(f"{__name__}: Critical error in alert processing for tx {tx.hash}: {str(e)}")
            return []

    def get_alert_types(self) -> List[str]:
        """Get list of available alert types"""
        return list(self.alert_processors.keys())

    def get_processor(self, alert_type: str):
        """Get specific alert processor by type"""
        return self.alert_processors.get(alert_type)
