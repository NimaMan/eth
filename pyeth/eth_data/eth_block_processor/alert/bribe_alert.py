from typing import List, Any

from eth_block_processor.utils.common_addresses import fee_recipients
from eth_block_processor.alert.base_alert_class import BaseAlert
from eth_block_processor.data_models.alert_models import BribeAlertData
from eth_block_processor.data_models.txn_models import ProcessedTransaction
from eth_block_processor.alert.config import bribe_threshold
from eth_block_processor.utils.logger import get_logger


logger = get_logger("bribe_alert", log_folder="alert")


class BribeAlert(BaseAlert):
    def __init__(self):
        self.fee_recipients = set(fee_recipients.keys())
        self.bribe_threshold = bribe_threshold
        
    def _is_alert(self, detailed_txn: ProcessedTransaction) -> bool:
        if detailed_txn.bribe_amount > self.bribe_threshold:
            return True, detailed_txn.bribe_amount
        return False, 0.0
        
    async def process_txn(self, detailed_txn: ProcessedTransaction) -> List[BribeAlertData]:
        """Process a block to detect potential bribe events"""
        is_bribe, bribe_value = self._is_alert(detailed_txn)
        if is_bribe:
            alert_data = self.create_alert(detailed_txn, bribe_value)
            self.send_alert(alert_data)
            return [alert_data]
        return []
    
    def create_alert(self, txn: ProcessedTransaction, bribe_value: float) -> BribeAlertData:
        """Create a bribe alert from transaction data"""
        return BribeAlertData(
            block_number=txn.block_number,
            transaction_hash=txn.hash,
            from_address=txn.from_address,
            value=txn.value,
            bribe_amount=bribe_value,
            erc20_contracts=tuple(txn.erc20_contracts),
        )
    
    def send_alert(self, alert_data: BribeAlertData) -> None:
        """Send/log the bribe alert"""
        logger.info(f"Bribe-> Txn: {alert_data.transaction_hash} "
                    f"From: {alert_data.from_address} "
                    f"Value: {alert_data.bribe_amount}"
                    )


