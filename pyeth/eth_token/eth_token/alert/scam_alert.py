from dataclasses import dataclass
from typing import List, Set, Dict
from collections import OrderedDict, defaultdict
from eth_token.alert.base_alert import BaseAlert
from eth_token.live_erc20_token.live_token import LiveERC20Token
from eth_token.utils.logger import get_logger


logger = get_logger("scam_tokens", log_folder="alert")


@dataclass
class ScamAlertData:
    transaction_hash: str
    block_number: int
    contract_address: str
    reason: str
    confidence: float
    involved_addresses: Set[str]
    alert_type: str = "Scam"    


class ScamAlert(BaseAlert):
    """Alert for detected scam patterns"""
    def __init__(self):
        super().__init__()
        # Track all alerts per token: token_address -> OrderedDict[txn_hash -> alert_data]
        self._token_alerts: Dict[str, OrderedDict[str, ScamAlertData]] = defaultdict(OrderedDict)
        
    def _is_alert(self, live_erc20_token: LiveERC20Token) -> bool:
        """Check if token has been flagged for scam activity"""
        assessment = live_erc20_token.latest_token_assessment
        if not assessment:
            return False
            
        scam_data = assessment.get('scam_assessment', {})
        txn_hash = scam_data.get('transaction_hash', '')
        contract_address = live_erc20_token.contract_address
        
        # If we've already alerted on this txn, skip
        if txn_hash in self._token_alerts[contract_address]:
            return False
            
        # If it's a scam, alert
        if scam_data.get('is_scam', False):
            return True
            
        return False
        
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
        # Store alert in token's history
        self._token_alerts[alert_data.contract_address][alert_data.transaction_hash] = alert_data
        
        logger.info(f"{alert_data}")
        
    @property
    def latest_alerts(self) -> Dict[str, ScamAlertData]:
        """Get the most recent alert for each token"""
        return {
            token: next(reversed(alerts.values())) if alerts else None
            for token, alerts in self._token_alerts.items()
        }
