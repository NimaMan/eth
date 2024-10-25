import pandas as pd
from ethblockprocessor.data_models.txn_models import DetailedTransaction, TransactionType
from ethblockprocessor.data_models.alert_models import TradingEnabledAlertData
from ethblockprocessor.utils.logger import get_logger


logger = get_logger()


class TradingEnabledAlert:
    def __init__(self):
        #self.alert_path = "ethblockprocessor/logs/trading_enabled_alert.csv"
        #self.df = pd.read_csv(self.alert_path)
        pass 
    
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
        logger.info(f"Trading enabled alert sent: {alert_data}")
