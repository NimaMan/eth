
import asyncio
from typing import List
from eth_block_processor.alert.trading_enabled_alert import TradingEnabledAlert
from eth_block_processor.alert.bribe_alert import BribeAlert
from eth_block_processor.alert.contract_creation_alert import ContractCreationAlert
from eth_block_processor.alert.user_involved_alert import OrcaAlert, WhaleAlert
from eth_block_processor.data_models.txn_models import ProcessedTransaction
from eth_block_processor.utils.logger import get_logger


logger = get_logger("alert_processor", log_folder="alert")


class TransactionAlertProcessor:
    def __init__(self):
        # Initialize all alert processors
        self.alert_processors = {
            'trading_enabled': TradingEnabledAlert(),
            'bribe': BribeAlert(),
            'contract_creation': ContractCreationAlert(),
            'orca': OrcaAlert(),
            'whale': WhaleAlert()
        }

    async def process_single_alert(self, alert_type: str, processor, txn: ProcessedTransaction):
        """Process a single alert type asynchronously"""
        try:
            alerts = await processor.process_txn(txn)
            if alerts:
                logger.debug(f"Generated {alert_type} alerts for tx {txn.hash}: {len(alerts)}")
            return alerts
        except Exception as e:
            logger.error(f"{__name__}: Error processing {alert_type} alert for tx {txn.hash}: {str(e)}")
            return []

    async def process_transaction(self, txn: ProcessedTransaction):
        """
        Process a transaction through all alert processors concurrently
        
        Args:
            txn: Detailed transaction to process
            
        Returns:
            List of alerts generated from all processors
        """
        try:
            # Create tasks for all alert processors
            alert_tasks = [
                self.process_single_alert(alert_type, processor, txn)
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
                    logger.error(f"{__name__}: Alert processing error for tx {txn.hash}: {str(result)}")
            return all_alerts
            
        except Exception as e:
            logger.error(f"{__name__}: Critical error in alert processing for tx {txn.hash}: {str(e)}")
            return []

    def get_alert_types(self) -> List[str]:
        """Get list of available alert types"""
        return list(self.alert_processors.keys())

    def get_processor(self, alert_type: str):
        """Get specific alert processor by type"""
        return self.alert_processors.get(alert_type)
