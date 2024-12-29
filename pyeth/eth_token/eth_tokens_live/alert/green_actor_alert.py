from dataclasses import dataclass
from typing import List, Set, Tuple
from eth_tokens_live.alert.base_alert_class import BaseAlert
from eth_tokens_live.live_erc20_token.live_token import LiveERC20Token
from eth_tokens_live.utils.logger import get_logger


logger = get_logger("green_tokens", log_folder="alert")


@dataclass
class GreenActorAlertData:
    contract_address: str
    block_number: int
    transaction_hash: str
    involved_addresses: Set[str]
    alert_type: str = "Green Actor"
    action: str = ""


class GreenActorAlert(BaseAlert):
    """Alert for legitimate trading activity from green actors"""
    def __init__(self):
        super().__init__()
        
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
            
        return True, "Swap"    
        
    def create_alert(self, live_erc20_token: LiveERC20Token, action: str) -> GreenActorAlertData:
        assessment = live_erc20_token.latest_token_assessment
        green_assessment = assessment.get('green_assessment', {})
        
        return GreenActorAlertData(
            block_number=live_erc20_token.latest_block_number,
            contract_address=live_erc20_token.contract_address,
            alert_type=f"Green Actor {action}",
            involved_addresses=green_assessment.get('green_actors', set()),
            transaction_hash=assessment.get('transaction_hash', ''),
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
        logger.info(
            f"Green Actor {alert_data.action}: {alert_data.contract_address} "
            f"Addresses: {alert_data.involved_addresses} "
            f"Txn: {alert_data.transaction_hash}"
        )
