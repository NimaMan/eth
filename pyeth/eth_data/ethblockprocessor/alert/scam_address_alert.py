
from ethblockprocessor.alert.base_alert_class import BaseAlert
from ethblockprocessor.data_models.txn_models import DetailedTransaction
from ethblockprocessor.data_models.alert_models import ScamAddressAlertData
from ethblockprocessor.alert.config import scam_address_threshold


class ScamAddressAlert(BaseAlert):
    def __init__(self, scam_address_threshold: float = scam_address_threshold):
        self.scam_address_threshold = scam_address_threshold

    def get_alert(self, txn: DetailedTransaction):
        if txn.to_address in self.scam_address_threshold:
            alert_data = self.create_alert(txn)
            self.send_alert(alert_data)
            return [alert_data]
        return []

    def create_alert(self, txn: DetailedTransaction):
        alert_data = ScamAddressAlertData(
            block_number=txn.block_number,
            transaction_hash=txn.hash.hex(),
            from_address=txn.from_address,
            to_address=txn.to_address,
            value=txn.value,
            alert_type="ScamAddress",
        )
        return alert_data

    def send_alert(self, alert_data: ScamAddressAlertData):
        print(alert_data)
