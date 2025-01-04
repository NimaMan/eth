from typing import Dict
import pandas as pd
from eth_token_monitor.live_erc20_token.data.live_token_data import LiveTokenData


class ERC20TokenAlertData:
    
    def __init__(self, live_token_data: LiveTokenData):
        self.live_token_data = live_token_data

    def to_dataframe(self) -> pd.DataFrame:
        alerts_df = pd.DataFrame([self._serialize_name(alert) for alert in self.live_token_data.alerts])
        return alerts_df
    
    def _serialize_name(self, alert: Dict) -> dict:
        alert_dict = {
            'id': alert['id'],
            'block': alert['block_number'],
            'sent': alert['sent'],
        }
        return alert_dict