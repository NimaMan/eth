from typing import List, Any, Optional, Set
from eth_tokens_live.alert.base_alert_class import BaseAlert, AlertPriority
from eth_tokens_live.live_erc20_token.live_token import LiveERC20Token
from eth_tokens_live.alert.alert_data_models import UserInvolvedAlertData
from eth_tokens_live.alert.config import get_grey_addresses, get_orca_addresses, get_whale_addresses
from eth_tokens_live.utils.logger import get_logger


class BaseUserAlert(BaseAlert):
    """Base class for user-related alerts to avoid code duplication"""
    def __init__(self, addresses_getter):
        super().__init__()
        self.scam_logger = get_logger(f"scam", log_folder="alert")
        self.green_logger = get_logger(f"green", log_folder="alert")
        self.addresses_set: Set[str] = addresses_getter()
        self.alert_type = None

    def _is_alert(self, live_erc20_token: LiveERC20Token) -> bool:
        # Check both from and to addresses, including contract interactions
        involved_addresses = set(live_erc20_token.unique_addresses)
        return bool(involved_addresses & self.addresses_set)
    
    async def process_token(self, live_erc20_token: LiveERC20Token) -> List[UserInvolvedAlertData]:
        if self._is_alert(live_erc20_token):
            alert_data = self.create_alert(live_erc20_token)
            self.send_alert(alert_data)
            return [alert_data]
        return []

    def create_alert(self, live_erc20_token: LiveERC20Token) -> UserInvolvedAlertData:
        involved_addresses = self.addresses_set & live_erc20_token.unique_addresses
        return UserInvolvedAlertData(
            block_number=live_erc20_token.block_number,
            involved_addresses=list(involved_addresses),
            alert_type=self.alert_type,
            erc20_contracts=tuple(live_erc20_token.erc20_contracts),
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



