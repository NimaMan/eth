from typing import List, Any, Optional, Set
from eth_block_processor.alert.base_alert_class import BaseAlert, AlertPriority
from eth_block_processor.data_models.txn_models import DetailedTransaction
from eth_block_processor.data_models.alert_models import UserInvolvedAlertData
from eth_block_processor.alert.config import get_grey_addresses, get_orca_addresses, get_whale_addresses
from eth_block_processor.utils.logger import get_logger


class BaseUserAlert(BaseAlert):
    """Base class for user-related alerts to avoid code duplication"""
    def __init__(self, alert_type: str, addresses_getter):
        super().__init__()
        self.alert_type = alert_type
        self.logger = get_logger(f"{alert_type}_alert", log_folder="alert")
        self.addresses_set: Set[str] = addresses_getter()

    def _is_alert(self, detailed_txn: DetailedTransaction) -> bool:
        # Check both from and to addresses, including contract interactions
        involved_addresses = set(detailed_txn.unique_addresses)
        return bool(involved_addresses & self.addresses_set)
    
    async def process_txn(self, detailed_txn: DetailedTransaction) -> List[UserInvolvedAlertData]:
        if self._is_alert(detailed_txn):
            alert_data = self.create_alert(detailed_txn)
            self.send_alert(alert_data)
            return [alert_data]
        return []

    def create_alert(self, txn: DetailedTransaction) -> UserInvolvedAlertData:
        involved_addresses = self.addresses_set & txn.unique_addresses
        return UserInvolvedAlertData(
            block_number=txn.block_number,
            transaction_hash=txn.hash,
            from_address=txn.from_address,
            involved_addresses=list(involved_addresses),
            txn_type=txn.txn_type,
            alert_type=self.alert_type,
            erc20_contracts=tuple(txn.erc20_contracts),
        )

    def send_alert(self, alert_data: UserInvolvedAlertData) -> None:
        self.logger.info(
            f"{self.alert_type} -> Txn: {alert_data.transaction_hash} "
            f"addresses: {alert_data.involved_addresses}"
        )


class GreyAddressAlert(BaseUserAlert):
    """Malicious addresses"""
    def __init__(self):
        super().__init__("Grey", get_grey_addresses)
        self.priority = AlertPriority.HIGH


class OrcaAlert(BaseUserAlert):
    """Known trading addresses"""
    def __init__(self):
        super().__init__("Orca", get_orca_addresses)
        self.priority = AlertPriority.MEDIUM


class WhaleAlert(BaseUserAlert):
    """Large holder addresses"""
    def __init__(self):
        super().__init__("Whale", get_whale_addresses)
        self.priority = AlertPriority.MEDIUM



