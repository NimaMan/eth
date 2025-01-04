from typing import Any, List, Dict, Optional
from abc import ABC, abstractmethod
from enum import IntEnum
from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token


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
    async def process_token(self, live_erc20_token: LiveERC20Token):
        """
        Process a single token to generate alerts, and send them to the alert system
        """
        pass

    @abstractmethod
    def create_alert(self, live_erc20_token: LiveERC20Token):
        """Create alert data from token"""
        pass

    @abstractmethod
    def send_alert(self, alert_data: Any):
        """Send/log the alert"""
        pass

    @abstractmethod
    def _is_alert(self, live_erc20_token: LiveERC20Token) -> bool:
        """Helper method to detect if the event is an alert"""
        pass