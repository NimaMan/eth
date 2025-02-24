from dataclasses import dataclass
from typing import List, Set, Tuple
from eth_token.alert.base_alert_class import BaseAlert
from eth_token.live_erc20_token.live_token import LiveERC20Token
from eth_token.utils.logger import get_logger


logger = get_logger("green_tokens", log_folder="alert")


@dataclass
class GreenActorAlertData:
    transaction_hash: str
    contract_address: str
    block_number: int
    involved_addresses: Set[str]
    alert_type: str = "Green Actor"
    action: str = ""


class GreenActorAlert(BaseAlert):
    """Alert for legitimate trading activity from green actors"""
    def __init__(self):
        super().__init__()
        self._last_alerts = {}  # Store last alert per token
        
    def _is_alert(self, live_erc20_token: LiveERC20Token) -> Tuple[bool, str]:
        """
        Check if transaction involves legitimate green actor trading
        Returns:
            Tuple[bool, str]: (is_alert, action_type)
        """
        assessment = live_erc20_token.latest_token_assessment
        if not assessment:
            return False, ""
            
        green_assessment = assessment.get('green_assessment', {})
        green_actors = green_assessment.get('green_actors', set())
        
        if not green_actors:
            return False, ""
            
        # Check if this is a duplicate alert
        last_alert = self._last_alerts.get(live_erc20_token.contract_address)
        if last_alert and last_alert.involved_addresses == green_actors:
            return False, ""
            
        return True, "Swap"    
        
    def create_alert(self, live_erc20_token: LiveERC20Token, action: str) -> GreenActorAlertData:
        assessment = live_erc20_token.latest_token_assessment
        green_assessment = assessment.get('green_assessment', {})
        green_actors_dict = green_assessment.get('green_actors', {})
        involved_addresses = green_actors_dict.values() # get all green actors
        
        # Get last transaction from OrderedDict
        latest_txn_hash = next(reversed(green_actors_dict))
        
        return GreenActorAlertData(
            transaction_hash=latest_txn_hash,
            contract_address=live_erc20_token.contract_address,
            block_number=live_erc20_token.latest_block_number,
            involved_addresses=involved_addresses,
            action=action,
        )

    async def process_token(self, live_erc20_token: LiveERC20Token) -> List[GreenActorAlertData]:
        is_alert, action = self._is_alert(live_erc20_token)
        
        if is_alert:
            alert_data = self.create_alert(live_erc20_token, action)
            self.send_alert(alert_data)
            return [alert_data]
        return []
    
    def send_alert(self, alert_data: GreenActorAlertData) -> None:
        # Store this alert as the last one for this token
        self._last_alerts[alert_data.contract_address] = alert_data
        
        logger.info(f"{alert_data}")
