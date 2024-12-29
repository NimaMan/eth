from dataclasses import dataclass
from typing import List, Set, Dict
from eth_tokens_live.alert.base_alert_class import BaseAlert
from eth_tokens_live.live_erc20_token.live_token import LiveERC20Token
from eth_tokens_live.utils.logger import get_logger


logger = get_logger("scam_tokens", log_folder="alert")


@dataclass
class ScamAlertData:
    block_number: int
    contract_address: str
    transaction_hash: str
    reason: str
    confidence: float
    involved_addresses: Set[str]
    alert_type: str = "Scam"    


class ScamAlert(BaseAlert):
    """Alert for detected scam patterns"""
    def __init__(self):
        super().__init__()
        self._last_alerts = {}  # Store last alert per token
        
    def _is_alert(self, live_erc20_token: LiveERC20Token) -> bool:
        """Check if token has been flagged for scam activity"""
        assessment = live_erc20_token.latest_token_assessment
        if not assessment:
            return False
            
        scam_data = assessment.get('scam_assessment', {})
        is_scam = scam_data.get('is_scam', False)
        
        if not is_scam:
            return False
            
        # Check if this is a duplicate alert
        last_alert = self._last_alerts.get(live_erc20_token.contract_address)
        if last_alert:
            current_data = {
                'reason': scam_data.get('reason', ''),
                'confidence': scam_data.get('confidence', 0),
                'involved_addresses': scam_data.get('involved_addresses', set())
            }
            last_data = {
                'reason': last_alert.reason,
                'confidence': last_alert.confidence,
                'involved_addresses': last_alert.involved_addresses
            }
            if current_data == last_data:
                return False
                
        return True
        
    def create_alert(self, live_erc20_token: LiveERC20Token) -> ScamAlertData:
        assessment = live_erc20_token.latest_token_assessment
        scam_data = assessment.get('scam_assessment', {})
        
        return ScamAlertData(
            block_number=live_erc20_token.latest_block_number,
            contract_address=live_erc20_token.contract_address,
            transaction_hash=scam_data.get('transaction_hash', ''),
            reason=scam_data.get('reason', ''),
            confidence=scam_data.get('confidence', 0),
            involved_addresses=scam_data.get('involved_addresses', set()),
        )

    async def process_token(self, live_erc20_token: LiveERC20Token) -> List[ScamAlertData]:
        if self._is_alert(live_erc20_token):
            alert_data = self.create_alert(live_erc20_token)
            self.send_alert(alert_data)
            return [alert_data]
        return []
    
    def send_alert(self, alert_data: ScamAlertData) -> None:
        # Store this alert as the last one for this token
        self._last_alerts[alert_data.contract_address] = alert_data
        
        logger.info(
            f"SCAM ALERT: {alert_data.alert_type}"
            f"Txn: {alert_data.transaction_hash}"
            f"Token: {alert_data.contract_address}"
            f"Reason: {alert_data.reason}"
            f"Confidence: {alert_data.confidence}"
            f"Involved Addresses: {alert_data.involved_addresses}\n"
        )