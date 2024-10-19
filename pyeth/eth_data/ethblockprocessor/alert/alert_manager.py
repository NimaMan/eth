from ethblockprocessor.data_models.txn_models import DetailedTransaction
from ethblockprocessor.alert.trading_enabled_alert import TradingEnabledAlert
from ethblockprocessor.alert.bribe_alert import BribeAlert
from ethblockprocessor.alert.contract_creation_alert import ContractCreationAlert


class AlertManager:
    def __init__(self):
        self.alerts = [
            TradingEnabledAlert,
            BribeAlert,
            ContractCreationAlert
        ]

        self.alert_objects = {alert.__name__: alert() for alert in self.alerts}

    def check_alerts(self, transaction: DetailedTransaction):
        triggered_alerts = []
        for alert_name, alert in self.alert_objects.items():
            alerts = alert.get_alert(transaction)
            if len(alerts) > 0:
                triggered_alerts.extend(alerts)
        return triggered_alerts
