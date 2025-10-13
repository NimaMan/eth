from typing import List, Any

from eth_data.chain_utils.common_addresses import fee_recipients
from eth_data.tx_alert.base_alert_class import BaseAlert
from eth_data.tx_alert.alert_models import BribeAlertData
from eth_data.tx_processor.data_models.tx_models import ProcessedTransaction
from eth_data.tx_alert.config import bribe_threshold
from eth_data.utils.logger import get_logger


logger = get_logger("bribe_alert", log_folder="alert")


class BribeAlert(BaseAlert):
    def __init__(self):
        self.fee_recipients = set(fee_recipients.keys())
        self.bribe_threshold = bribe_threshold
        
    def _is_alert(self, detailed_tx: ProcessedTransaction) -> bool:
        if detailed_tx.bribe_amount > self.bribe_threshold:
            return True, detailed_tx.bribe_amount
        return False, 0.0
        
    async def process_tx(self, detailed_tx: ProcessedTransaction) -> List[BribeAlertData]:
        """Process a block to detect potential bribe events"""
        is_bribe, bribe_value = self._is_alert(detailed_tx)
        if is_bribe:
            alert_data = self.create_alert(detailed_tx, bribe_value)
            self.send_alert(alert_data)
            return [alert_data]
        return []
    
    def create_alert(self, tx: ProcessedTransaction, bribe_value: float) -> BribeAlertData:
        """Create a bribe alert from transaction data"""
        return BribeAlertData(
            block_number=tx.block_number,
            transaction_hash=tx.hash,
            from_address=tx.from_address,
            value=tx.value,
            bribe_amount=bribe_value,
            erc20_contracts=tuple(tx.erc20_contracts),
        )
    
    def send_alert(self, alert_data: BribeAlertData) -> None:
        """Send/log the bribe alert"""
        logger.info(f"Bribe-> tx: {alert_data.transaction_hash} "
                    f"From: {alert_data.from_address} "
                    f"Value: {alert_data.bribe_amount}"
                    )


