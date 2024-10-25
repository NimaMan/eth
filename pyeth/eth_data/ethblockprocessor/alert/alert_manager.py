import asyncio
from ethblockprocessor.data_models.txn_models import DetailedTransaction
from ethblockprocessor.alert.trading_enabled_alert import TradingEnabledAlert
from ethblockprocessor.alert.bribe_alert import BribeAlert
from ethblockprocessor.alert.contract_creation_alert import ContractCreationAlert
from ethblockprocessor.alert.user_involved_alert import GreyAddressAlert
from ethblockprocessor.utils.logger import get_logger
from ethblockprocessor.utils.profiler import profile


logger = get_logger()


class AlertManager:
    def __init__(self):
        self.trading_enabled_alert = TradingEnabledAlert()
        self.bribe_alert = BribeAlert()
        self.contract_creation_alert = ContractCreationAlert()
        self.grey_address_alert = GreyAddressAlert()
        self.alert_objects = [
            self.trading_enabled_alert,
            self.bribe_alert,
            self.contract_creation_alert,
            self.grey_address_alert
        ]

    def check_alerts(self, transaction: DetailedTransaction):
        triggered_alerts = []
        triggered_alerts.extend(self.check_trading_enabled_alert(transaction))
        triggered_alerts.extend(self.check_bribe_alert(transaction))
        triggered_alerts.extend(self.check_contract_creation_alert(transaction))
        triggered_alerts.extend(self.check_grey_address_alert(transaction))
        return triggered_alerts
    
    @profile
    def check_trading_enabled_alert(self, transaction: DetailedTransaction):
        return self.trading_enabled_alert.get_alert(transaction)
    
    @profile
    def check_bribe_alert(self, transaction: DetailedTransaction):
        return self.bribe_alert.get_alert(transaction)
    
    @profile
    def check_contract_creation_alert(self, transaction: DetailedTransaction):
        return self.contract_creation_alert.get_alert(transaction)
    
    @profile
    def check_grey_address_alert(self, transaction: DetailedTransaction):
        return self.grey_address_alert.get_alert(transaction)
    
    async def check_alerts_async(self, transaction: DetailedTransaction):
        tasks = []
        for alert in self.alert_objects:
            if asyncio.iscoroutinefunction(alert.get_alert):
                tasks.append(asyncio.create_task(alert.get_alert(transaction)))
            else:
                tasks.append(asyncio.to_thread(alert.get_alert, transaction))
        results = await asyncio.gather(*tasks)
        triggered_alerts = [alert for sublist in results for alert in sublist if sublist]
        return triggered_alerts
