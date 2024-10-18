import pandas as pd
from ethblockprocessor.data_models.txn_models import DetailedTransaction, TransactionType
from ethblockprocessor.data_models.alert_models import TradingEnabledAlertData


class TradingEnabledAlert:
    def __init__(self):
        #self.alert_path = "ethblockprocessor/logs/trading_enabled_alert.csv"
        #self.df = pd.read_csv(self.alert_path)
        pass 
    
    def create_alert(self, txn: DetailedTransaction):
        alert_data = TradingEnabledAlertData(
            block_number=txn.block_number,
            hash=txn.hash,
            txn_index=txn.txn_index,
            from_address=txn.from_address,
            to_address=txn.to_address,
            value=txn.value
        )
        return alert_data
        
    def check_trading_enabled(self, txn: DetailedTransaction):
        if txn.txn_type in ["OpenTrading", "OpenTradingV2"]:
            self.df.loc[len(self.df)] = [txn.block_number, txn.hash, txn.txn_index, txn.from_address, txn.to_address, txn.value]
            self.df.to_csv(self.alert_path, index=False)
            return True
        return False
    
    def check_trading_enabled_v2(self, txn: DetailedTransaction):
        if txn.txn_type == "OpenTradingV2":
            self.df.loc[len(self.df)] = [txn.block_number, txn.hash, txn.txn_index, txn.from_address, txn.to_address, txn.value]
            self.df.to_csv(self.alert_path, index=False)
            return True
        return False
    
    def send_alert(self, txn: DetailedTransaction):
        if self.check_trading_enabled(txn) or self.check_trading_enabled_v2(txn):
            print(f"Trading enabled alert sent for {txn.from_address} at block {txn.block_number}")
            return True
        return False
