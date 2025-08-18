from typing import List, Any, Optional, Set
from eth_data.tx_alert.base_alert_class import BaseAlert, AlertPriority
from eth_data.tx_processor.data_models.txn_models import ProcessedTransaction
from eth_data.tx_alert.alert_models import UserInvolvedAlertData
from eth_data.tx_alert.config import get_grey_addresses, get_orca_addresses, get_whale_addresses
from eth_data.utils.logger import get_logger


class BaseUserAlert(BaseAlert):
    """Base class for user-related alerts to avoid code duplication"""
    def __init__(self, addresses_getter):
        super().__init__()
        self.scam_logger = get_logger(f"scam", log_folder="alert")
        self.green_logger = get_logger(f"green", log_folder="alert")
        self.addresses_set: Set[str] = addresses_getter()
        self.alert_type = None

    def _is_alert(self, detailed_txn: ProcessedTransaction) -> bool:
        # Check both from and to addresses, including contract interactions
        involved_addresses = set(detailed_txn.unique_addresses)
        return bool(involved_addresses & self.addresses_set)
    
    async def process_txn(self, detailed_txn: ProcessedTransaction) -> List[UserInvolvedAlertData]:
        if self._is_alert(detailed_txn):
            alert_data = self.create_alert(detailed_txn)
            self.send_alert(alert_data)
            return [alert_data]
        return []

    def create_alert(self, txn: ProcessedTransaction) -> UserInvolvedAlertData:
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
        if "Scam" in self.alert_type:
            self.scam_logger.info(
                f"{self.alert_type} -> Txn: {alert_data.transaction_hash} "
                f"addresses: {alert_data.involved_addresses}"
            )
        else:
            self.green_logger.info(
                f"{self.alert_type} -> Txn: {alert_data.transaction_hash} "
                f"addresses: {alert_data.involved_addresses}"
            )


class GreyAddressAlert(BaseUserAlert):
    """Malicious addresses"""
    def __init__(self):
        super().__init__(get_grey_addresses)
        self.priority = AlertPriority.HIGH
        self.alert_type = "Scam | Malicious Address"


class OrcaAlert(BaseUserAlert):
    """Known trading addresses"""
    def __init__(self):
        super().__init__(get_orca_addresses)
        self.priority = AlertPriority.MEDIUM

    async def process_txn(self, detailed_txn: ProcessedTransaction) -> List[UserInvolvedAlertData]:
        if self._is_alert(detailed_txn):
            # check if the txn has a swap action
            num_erc20_token_transfers = len(detailed_txn.erc20_contracts)
            if detailed_txn.txn_type == "Swap" or detailed_txn.txn_type == "Approve":
                self.alert_type = "Orca"
                alert_data = self.create_alert(detailed_txn)
                self.send_alert(alert_data)
                return [alert_data]
            
            elif num_erc20_token_transfers > 15 or len(detailed_txn.unique_addresses) > 15:
                self.alert_type = "Scam | Orca"
                alert_data = self.create_alert(detailed_txn)
                self.send_alert(alert_data)
                return [alert_data]
            else:
                self.alert_type = "Orca"
                alert_data = self.create_alert(detailed_txn)
                self.send_alert(alert_data)
                return [alert_data]
            
        return []
    

class WhaleAlert(BaseUserAlert):
    """Large holder addresses"""
    def __init__(self):
        super().__init__(get_whale_addresses)
        self.priority = AlertPriority.MEDIUM

    async def process_txn(self, detailed_txn: ProcessedTransaction) -> List[UserInvolvedAlertData]:
        if self._is_alert(detailed_txn):
            # check if the txn has a swap action
            num_erc20_token_transfers = len(detailed_txn.erc20_contracts)
            
            if detailed_txn.txn_type == "Swap" or detailed_txn.txn_type == "Approve":
                self.alert_type = "Orca"
                alert_data = self.create_alert(detailed_txn)
                self.send_alert(alert_data)
                return [alert_data]
            
            elif num_erc20_token_transfers > 15 or len(detailed_txn.unique_addresses) > 15:
                self.alert_type = "Scam | Orca"
                alert_data = self.create_alert(detailed_txn)
                self.send_alert(alert_data)
                return [alert_data]
            else:
                self.alert_type = "Orca"
                alert_data = self.create_alert(detailed_txn)
                self.send_alert(alert_data)
                return [alert_data]
            
        return []
    
