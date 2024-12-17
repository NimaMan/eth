import pandas as pd
from eth_block_processor.data_models.txn_models import DetailedTransaction
from eth_block_processor.data_models.alert_models import TradingEnabledAlertData
from eth_block_processor.utils.logger import get_logger
from eth_block_processor.alert.base_alert_class import BaseAlert
from web3.types import BlockData
from typing import List, Any

logger = get_logger("trading_enabled_alert", log_folder="alert")


class TradingEnabledAlert(BaseAlert):
    def __init__(self):
        #self.alert_path = "eth_block_processor/logs/trading_enabled_alert.csv"
        #self.df = pd.read_csv(self.alert_path)
        self.max_latency = 1.0  # 1 second max latency
        
    def _is_alert(self, tx: DetailedTransaction) -> bool:
        """
        Check if the transaction should trigger a trading enabled alert
        
        Args:
            tx: The transaction to check
            
        Returns:
            bool: True if transaction should trigger alert, False otherwise
        """
        return tx.txn_type in ["OpenTrading", "OpenTradingV2"]
        
    async def process(self, block: BlockData) -> List[Any]:
        """
        Process a block to detect trading enabled events
        
        Args:
            block: The block data to process
            
        Returns:
            List of trading enabled alerts
        """
        alerts = []
        # Implementation logic here
        # Example:
        for tx in block.transactions:
            # Add your trading enabled detection logic
            if self._is_trading_enabled_event(tx):
                alerts.append({
                    'type': 'trading_enabled',
                    'transaction_hash': tx.hash,
                    'block_number': block.number,
                    'timestamp': block.timestamp
                })
        
        return alerts
    
    def _is_trading_enabled_event(self, tx) -> bool:
        """Helper method to detect trading enabled events"""
        # Implement your detection logic here
        return False  # Placeholder
    
    def create_alert(self, txn: DetailedTransaction):
        alert_data = TradingEnabledAlertData(
            block_number=txn.block_number,
            transaction_hash=txn.hash.hex(),
            from_address=txn.from_address,
            contract_address=txn.to_address,
        )
        return alert_data
        
    def check_trading_enabled(self, txn: DetailedTransaction):
        if txn.txn_type in ["OpenTrading", "OpenTradingV2"]:
            alert_data = self.create_alert(txn)
            return [alert_data]
        return []
    
    def check_trading_enabled_v2(self, txn: DetailedTransaction):
        if txn.txn_type == "OpenTradingV2":
            alert_data = self.create_alert(txn)
            return [alert_data]
        return []
    
    def get_alert(self, txn: DetailedTransaction):
        if self.check_trading_enabled(txn) or self.check_trading_enabled_v2(txn):
            alert_data = self.create_alert(txn)
            self.send_alert(alert_data)
            return [alert_data]
        return []

    def send_alert(self, alert_data: TradingEnabledAlertData):
        logger.info(f"{alert_data}")
