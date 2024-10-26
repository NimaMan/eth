import asyncio
from concurrent.futures import ThreadPoolExecutor
from typing import List, Dict, Any, Tuple
from ethblockprocessor.data_models.txn_models import DetailedTransaction
from ethblockprocessor.alert.trading_enabled_alert import TradingEnabledAlert
from ethblockprocessor.alert.bribe_alert import BribeAlert
from ethblockprocessor.alert.contract_creation_alert import ContractCreationAlert
from ethblockprocessor.alert.user_involved_alert import GreyAddressAlert


ALERT_CLASSES = {
    'trading_enabled': TradingEnabledAlert,
    'bribe': BribeAlert,
    'contract_creation': ContractCreationAlert,
    'grey_address': GreyAddressAlert
}


class AlertManager:
    def __init__(self, max_workers: int = 4):
        self.thread_executor = ThreadPoolExecutor(max_workers=max_workers)
        
    async def check_alerts_async(self, transaction: DetailedTransaction):
        # Execute alerts in parallel threads
        loop = asyncio.get_event_loop()
        futures = [
            loop.run_in_executor(self.thread_executor, self._process_alert, name, transaction)
            for name in ALERT_CLASSES.keys()
        ]
        
        triggered_alerts = []
        results = await asyncio.gather(*futures, return_exceptions=True)
        for result in results:
            if isinstance(result, Exception):
                # Handle individual alert exceptions if necessary
                continue
            if result:
                triggered_alerts.extend(result)
        
        return triggered_alerts

    @staticmethod
    def _process_alert(alert_name: str, transaction: DetailedTransaction):
        alert = ALERT_CLASSES[alert_name]()
        return alert.get_alert(transaction)

    def __del__(self):
        self.thread_executor.shutdown(wait=True)
