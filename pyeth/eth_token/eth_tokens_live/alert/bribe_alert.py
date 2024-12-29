from collections.abc import Set
from dataclasses import dataclass, field
from typing import List, Any

from eth_tokens_live.alert.base_alert_class import BaseAlert
from eth_tokens_live.live_erc20_token.live_token import LiveERC20Token
from eth_tokens_live.alert.config import bribe_threshold
from eth_tokens_live.utils.logger import get_logger


logger = get_logger("tokens_bribe", log_folder="alert")


@dataclass
class BribeAlertData:
    block_number: int
    bribe_amount: float
    contract_address: str
    alert_type: str = "Bribe"


class BribeAlert(BaseAlert):
    def __init__(self):
        self.bribe_threshold = bribe_threshold
        
    def _is_alert(self, live_erc20_token: LiveERC20Token) -> bool:
        if live_erc20_token.token_data.total_bribe_amount > self.bribe_threshold:
            return True, live_erc20_token.token_data.total_bribe_amount
        return False, 0
        
    async def process_token(self, live_erc20_token: LiveERC20Token) -> List[BribeAlertData]:
        """Process a block to detect potential bribe events"""
        is_bribe, bribe_value = self._is_alert(live_erc20_token)
        if is_bribe:
            alert_data = self.create_alert(live_erc20_token, bribe_value)
            self.send_alert(alert_data)
            return [alert_data]
        return []
    
    def create_alert(self, live_erc20_token: LiveERC20Token, bribe_value: float) -> BribeAlertData:
        """Create a bribe alert from token data"""
        return BribeAlertData(
            block_number=live_erc20_token.block_number,
            bribe_amount=bribe_value,
            contract_address=live_erc20_token.contract_address,
        )
    
    def send_alert(self, alert_data: BribeAlertData) -> None:
        """Send/log the bribe alert"""
        logger.info(f"Bribe Alert: {alert_data.bribe_amount}")