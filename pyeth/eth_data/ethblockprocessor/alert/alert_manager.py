from ethblockprocessor.data_models.txn_models import DetailedTransaction
from ethblockprocessor.alert.trading_enabled_alert import TradingEnabledAlert
from ethblockprocessor.alert.bribe_alert import BribeAlert
from ethblockprocessor.alert.contract_creation_alert import ContractCreationAlert
from ethblockprocessor.alert.alert_db import AlertDB
import asyncio


class AlertManager:
    def __init__(self, save_alert_db: bool = True):
        self.alerts = [
            TradingEnabledAlert,
            BribeAlert,
            ContractCreationAlert,
        ]

        self.alert_objects = {alert.__name__: alert() for alert in self.alerts}
        self.save_alert_db = save_alert_db

    def check_alerts(self, transaction: DetailedTransaction):
        triggered_alerts = []
        for alert_name, alert in self.alert_objects.items():
            alerts = alert.get_alert(transaction)
            if len(alerts) > 0:
                triggered_alerts.extend(alerts)
                for triggered_alert in alerts:
                    self.alert_db.add_alert(triggered_alert)
        return triggered_alerts

    def save_alerts_to_db(self, alerts):
        if len(alerts) > 0:
            with AlertDB() as alert_db:
                alert_db.add_alerts(alerts)

    async def check_alerts_async(self, transaction: DetailedTransaction):
        if self.save_alert_db:
            tasks = []
            for alert in self.alert_objects.values():
                if asyncio.iscoroutinefunction(alert.get_alert):
                    tasks.append(asyncio.create_task(alert.get_alert(transaction)))
                else:
                    tasks.append(asyncio.to_thread(alert.get_alert, transaction))
            results = await asyncio.gather(*tasks)
            triggered_alerts = [alert for sublist in results for alert in sublist if sublist]
            self.save_alerts_to_db(triggered_alerts)
        else:
            triggered_alerts = self.check_alerts(transaction)
        return triggered_alerts
