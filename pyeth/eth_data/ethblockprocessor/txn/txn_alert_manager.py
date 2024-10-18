from web3 import Web3
from typing import List, Dict, Any
from ethblockprocessor.txn.txn_analyzer import TransactionAnalyzer
from ethblockprocessor.alert.alert_manager import AlertManager
from ethblockprocessor.data_models.txn_models import DetailedTransaction



class TransactionAlertManager:
    def __init__(self, w3: Web3):
        self.w3 = w3
        self.transaction_analyzer = TransactionAnalyzer(w3)
        self.alert_manager = AlertManager()

    async def analyze_and_check_alerts(self, transaction: DetailedTransaction):
        analazed_txn = await self.transaction_analyzer.analyze_transaction(transaction)
        alerts = self.alert_manager.check_alerts(analazed_txn)
        return alerts

    def send_alerts(self, alerts):
        self.alert_manager.send_alerts(alerts)