from typing import Any, List
from abc import ABC, abstractmethod
from enum import IntEnum
from eth_block_processor.data_models.txn_models import DetailedTransaction
from web3.types import BlockData

class AlertPriority(IntEnum):
    CRITICAL = 0  # Block processing
    HIGH = 1      # Quick alerts
    MEDIUM = 2    # Complex analysis
    LOW = 3       # Storage & maintenance

class BaseAlert(ABC):
    def __init__(self):
        self.priority = AlertPriority.HIGH  # Default to high priority
        self.max_latency = 1.0  # Default to 1 second max latency

    @abstractmethod
    async def process(self, block: BlockData) -> List[Any]:
        """Process a block and return any alerts"""
        pass

    @abstractmethod
    async def get_alert(self, transaction: DetailedTransaction):
        """Process a single transaction and return alerts"""
        pass

    @abstractmethod
    def create_alert(self, transaction: DetailedTransaction):
        """Create alert data from transaction"""
        pass

    @abstractmethod
    def send_alert(self, alert_data: Any):
        """Send/log the alert"""
        pass

    @abstractmethod
    def _is_alert(self, tx) -> bool:
        """Helper method to detect if the event is an alert"""
        # Implement your detection logic here
        return False  # Placeholder