import pandas as pd
from eth_block_processor.data_models.txn_models import ProcessedTransaction
from eth_block_processor.data_models.alert_models import TradingEnabledAlertData
from eth_block_processor.utils.logger import get_logger
from eth_block_processor.alert.base_alert_class import BaseAlert
from typing import List, Any


logger = get_logger("trading_enabled_alert", log_folder="alert")


class TradingEnabledAlert(BaseAlert):
    def __init__(self):
        pass
    
    def _is_alert(self, detailed_txn: ProcessedTransaction) -> bool:
        """
        Check if the transaction should trigger a trading enabled alert
        """
        if detailed_txn.txn_type == "Trading Enabled" or len(detailed_txn.trading_enabled_events) > 0:
            return True
        return False
        
    async def process_txn(self, detailed_txn: ProcessedTransaction) -> List[TradingEnabledAlertData]:
        """
        Process a block to detect trading enabled events
        
        Args:
            detailed_txn: The detailed transaction to process
            
        Returns:
            List of trading enabled alerts
        """
        if self._is_alert(detailed_txn):
            alert_data = self.create_alert(detailed_txn)
            self.send_alert(alert_data)
            return [alert_data]
        return []
    
    def create_alert(self, detailed_txn: ProcessedTransaction) -> TradingEnabledAlertData:
        alert_data = TradingEnabledAlertData(
            block_number=detailed_txn.block_number,
            transaction_hash=detailed_txn.hash,
            from_address=detailed_txn.from_address,
            contract_address=detailed_txn.to_address,
            erc20_contracts=tuple(detailed_txn.erc20_contracts),
        )
        return alert_data
        
    def send_alert(self, alert_data: TradingEnabledAlertData):
        logger.info(f"{alert_data}")
