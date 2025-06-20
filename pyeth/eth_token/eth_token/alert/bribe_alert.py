from dataclasses import dataclass
from typing import List

from eth_token.alert.base_alert import BaseAlert
from eth_token.erc20_token.erc20_token import ERC20Token
from eth_token.alert.config import bribe_threshold
from eth_token.utils.logger import get_logger


@dataclass
class BribeAlertData:
    block_number: int
    contract_address: str
    bribe_amount: float
    alert_type: str = "Bribe"


class BribeAlert(BaseAlert):
    def __init__(self):
        self.bribe_threshold = bribe_threshold
        self.min_change_threshold = 0.1  # Minimum 1% change to trigger new alert
        self._last_alert_values = {}  # Track last alert value per token
        self._last_alert_blocks = {}  # Track last alert block per token
        self.logger = get_logger("tokens_bribe", log_folder="alert")
        
    def _is_alert(self, live_erc20_token: ERC20Token) -> tuple[bool, float]:
        bribe_amount = live_erc20_token.token_data.total_bribe_amount
        token_address = live_erc20_token.contract_address
        current_block = live_erc20_token.token_data.latest_block_number
        
        if bribe_amount <= self.bribe_threshold:
            return False, 0
            
        # Check if we've already alerted for this block
        if (token_address in self._last_alert_blocks and 
            current_block == self._last_alert_blocks[token_address]):
            return False, 0
            
        # Check if value has changed enough to warrant new alert
        if token_address in self._last_alert_values:
            last_value = self._last_alert_values[token_address]
            percent_change = abs(bribe_amount - last_value) / last_value
            if percent_change < self.min_change_threshold:
                return False, 0
                
        # Update tracking and return alert
        self._last_alert_values[token_address] = bribe_amount
        self._last_alert_blocks[token_address] = current_block
        self.logger.debug(f"New alert triggered for {token_address} at block {current_block} with value {bribe_amount}")
        return True, bribe_amount
        
    async def process_token(self, live_erc20_token: ERC20Token) -> List[BribeAlertData]:
        """Process a token to detect potential bribe events"""
        is_bribe, bribe_value = self._is_alert(live_erc20_token)
        if is_bribe:
            alert_data = self.create_alert(live_erc20_token, bribe_value)
            self.send_alert(alert_data)
            return [alert_data]
        return []
    
    def create_alert(self, live_erc20_token: ERC20Token, bribe_value: float) -> BribeAlertData:
        """Create a bribe alert from token data"""
        return BribeAlertData(
            block_number=live_erc20_token.token_data.latest_block_number,
            contract_address=live_erc20_token.contract_address,
            bribe_amount=bribe_value,
        )   
    
    def send_alert(self, alert_data: BribeAlertData) -> None:
        """Send/log the bribe alert"""
        self.logger.info(f"{alert_data}")