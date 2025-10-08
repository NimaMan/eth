from eth_data.tx_processor.data_models import txn_models as tx_models
from eth_token.alert.base_alert import BaseAlert
from typing import List
from dataclasses import dataclass
from eth_token.utils.logger import get_logger

logger = get_logger("trading_enabled_alert", log_folder="alert")


@dataclass
class TradingEnabledAlertData:
    block_number: int
    transaction_hash: str
    from_address: str
    to_address: str

ProcessedTransaction = tx_models.ProcessedTransaction


def _tx_type(detailed_tx: ProcessedTransaction) -> str:
    """Handle legacy txn_type attribute during tx naming transition."""
    return getattr(detailed_tx, "tx_type", getattr(detailed_tx, "txn_type", ""))


class TradingEnabledAlert(BaseAlert):
    def __init__(self):
        pass
    
    def _is_alert(self, detailed_tx: ProcessedTransaction) -> bool:
        """
        Check if the transaction should trigger a trading enabled alert
        """
        tx_type = _tx_type(detailed_tx)
        if tx_type == "Trading Enabled" or len(detailed_tx.trading_enabled_events) > 0:
            return True
        return False
        
    async def process_tx(self, detailed_tx: ProcessedTransaction) -> List[TradingEnabledAlertData]:
        """
        Process a block to detect trading enabled events
        
        Args:
            detailed_tx: The detailed transaction to process
            
        Returns:
            List of trading enabled alerts
        """
        if self._is_alert(detailed_tx):
            alert_data = self.create_alert(detailed_tx)
            self.send_alert(alert_data)
            return [alert_data]
        return []
    
    def create_alert(self, detailed_tx: ProcessedTransaction) -> TradingEnabledAlertData:
        alert_data = TradingEnabledAlertData(
            block_number=detailed_tx.block_number,
            transaction_hash=detailed_tx.hash,
            from_address=detailed_tx.from_address,
            contract_address=detailed_tx.to_address,
            erc20_contracts=tuple(detailed_tx.erc20_contracts),
        )
        return alert_data
        
    def send_alert(self, alert_data: TradingEnabledAlertData):
        logger.info(f"{alert_data}")
