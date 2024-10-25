from typing import Set, List
from ethblockprocessor.data_models.txn_models import DetailedTransaction
from ethblockprocessor.data_models.alert_models import UserInvolvedAlertData
from ethblockprocessor.alert.base_alert_class import BaseAlert
from ethblockprocessor.alert.config import get_grey_addresses
from ethblockprocessor.utils.logger import get_logger


logger = get_logger("user_involved_alert")


class GreyAddressAlert(BaseAlert):
    def __init__(self):
        self.grey_addresses_set = get_grey_addresses()

    def get_alert(self, txn: DetailedTransaction):
        involved_addresses = txn.unique_addresses
        scammers = involved_addresses.intersection(self.grey_addresses_set)
        if scammers:
            alert_data = self.create_alert(txn, scammers)
            self.send_alert(alert_data)
            return [alert_data]
        return []

    def create_alert(self, txn: DetailedTransaction, involved_addresses: set):
        alert_data = UserInvolvedAlertData(
            block_number=txn.block_number,
            transaction_hash=txn.hash.hex(),
            from_address=txn.from_address,
            involved_addresses=list(involved_addresses),
            txn_type=txn.txn_type,
            alert_type="Grey Address",
        )
        return alert_data

    def send_alert(self, alert_data: UserInvolvedAlertData):
        logger.info(f"{alert_data}")
        
    def refresh(self):
        get_grey_addresses.cache_clear()
        self.grey_addresses_set = get_grey_addresses()
