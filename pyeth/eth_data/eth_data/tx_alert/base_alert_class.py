from typing import Any, List, Dict, Optional
from abc import ABC, abstractmethod
from enum import IntEnum
from eth_data.tx_processor.data_models.tx_models import ProcessedTransaction


class AlertPriority(IntEnum):
    CRITICAL = 0  # Block processing
    HIGH = 1      # Quick alerts
    MEDIUM = 2    # Complex analysis
    LOW = 3       # Storage & maintenance


class BaseAlert(ABC):
    """Base class for all alert processors"""
    
    def __init__(self):
        self.priority = AlertPriority.LOW

    @abstractmethod
    async def process_tx(self, detailed_tx: ProcessedTransaction):
        """
        Process a single transaction to generate alerts, and send them to the alert system
        """
        pass

    @abstractmethod
    def create_alert(self, detailed_tx: ProcessedTransaction):
        """Create alert data from transaction"""
        pass

    @abstractmethod
    def send_alert(self, alert_data: Any):
        """Send/log the alert"""
        pass

    @abstractmethod
    def _is_alert(self, tx: ProcessedTransaction) -> bool:
        """Helper method to detect if the event is an alert"""
        pass
