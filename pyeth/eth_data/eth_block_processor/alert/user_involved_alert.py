from typing import Set, List
from eth_block_processor.data_models.txn_models import DetailedTransaction
from eth_block_processor.data_models.alert_models import UserInvolvedAlertData
from eth_block_processor.alert.base_alert_class import BaseAlert
from eth_block_processor.alert.config import get_grey_addresses, get_orca_addresses, get_whale_addresses
from eth_block_processor.utils.logger import get_logger
from web3.types import BlockData
from typing import List, Any
from eth_block_processor.data_models.alert_models import UserInvolvedAlertData


class GreyAddressAlert(BaseAlert):
    def __init__(self):
        self.max_latency = 1.0  # 1 second max latency
        self.logger = get_logger("grey_address_alert", log_folder="alert")
        self.grey_addresses_set = get_grey_addresses()  # Initialize with actual addresses
        
    def _is_alert(self, tx: DetailedTransaction) -> bool:
        """
        Check if transaction involves a grey-listed address
        
        Args:
            tx: The transaction to check
            
        Returns:
            bool: True if transaction involves a grey-listed address
        """
        return (tx.from_address in self.grey_addresses_set or 
                tx.to_address in self.grey_addresses_set)
        
    async def process(self, block: BlockData) -> List[Any]:
        """
        Process a block to detect transactions involving grey-listed addresses
        
        Args:
            block: The block data to process
            
        Returns:
            List of grey address alerts
        """
        alerts = []
        for tx in block.transactions:
            if self._is_alert(tx):
                alert_data = self.create_alert(tx)
                alerts.append(alert_data)
        return alerts
    
    def create_alert(self, txn: DetailedTransaction) -> UserInvolvedAlertData:
        """Create an alert for grey address transaction"""
        return UserInvolvedAlertData(
            block_number=txn.block_number,
            transaction_hash=txn.hash.hex(),
            from_address=txn.from_address,
            to_address=txn.to_address,
            value=txn.value,
            timestamp=txn.timestamp
        )
    
    def get_alert(self, txn: DetailedTransaction) -> List[UserInvolvedAlertData]:
        """Check if transaction should trigger a grey address alert"""
        if self._is_alert(txn):
            alert_data = self.create_alert(txn)
            self.send_alert(alert_data)
            return [alert_data]
        return []

    def send_alert(self, alert_data: UserInvolvedAlertData) -> None:
        """Send/log the grey address alert"""
        self.logger.info(f"User Involved Alert: Transaction involving monitored address detected: {alert_data}")

    def refresh(self):
        """Refresh the grey addresses list"""
        get_grey_addresses.cache_clear()
        self.grey_addresses_set = get_grey_addresses()


class OrcaAlert(BaseAlert):
    def __init__(self):
        self.logger = get_logger("green_alert", log_folder="alert")
        self.orca_addresses_set = get_orca_addresses()

    def _is_alert(self, tx: DetailedTransaction) -> bool:
        return (tx.from_address in self.orca_addresses_set or 
                tx.to_address in self.orca_addresses_set)
    
    async def process(self, block: BlockData) -> List[Any]: 
        alerts = []
        for tx in block.transactions:
            if self._is_alert(tx):
                alert_data = self.create_alert(tx)
                alerts.append(alert_data)
        return alerts

    def create_alert(self, txn: DetailedTransaction) -> UserInvolvedAlertData:
        """Create an alert for orca transaction"""
        return UserInvolvedAlertData(
            block_number=txn.block_number,
            transaction_hash=txn.hash.hex(),
            from_address=txn.from_address,
            to_address=txn.to_address,
            value=txn.value,
            timestamp=txn.timestamp
        )

    def send_alert(self, alert_data: UserInvolvedAlertData) -> None:
        """Send/log the orca alert"""
        self.logger.info(f"Orca Alert: Transaction involving orca address detected: {alert_data}")


class WhaleAlert(BaseAlert):
    def __init__(self):
        self.logger = get_logger("green_alert", log_folder="alert")
        self.whale_addresses_set = get_whale_addresses()

    def _is_alert(self, tx: DetailedTransaction) -> bool:
        return (tx.from_address in self.whale_addresses_set or 
                tx.to_address in self.whale_addresses_set)
    
    async def process(self, block: BlockData) -> List[Any]: 
        alerts = []
        for tx in block.transactions:
            if self._is_alert(tx):
                alert_data = self.create_alert(tx)
                alerts.append(alert_data)
        return alerts
    
    def create_alert(self, txn: DetailedTransaction) -> UserInvolvedAlertData:
        """Create an alert for whale transaction"""
        return UserInvolvedAlertData(
            block_number=txn.block_number,
            transaction_hash=txn.hash.hex(),
            from_address=txn.from_address,
            to_address=txn.to_address,
            value=txn.value,
            timestamp=txn.timestamp
        )
    
    def send_alert(self, alert_data: UserInvolvedAlertData) -> None:
        """Send/log the whale alert"""
        self.logger.info(f"Whale Alert: Transaction involving whale address detected: {alert_data}")



