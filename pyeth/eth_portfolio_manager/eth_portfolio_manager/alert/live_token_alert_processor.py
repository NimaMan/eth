"""
AlertProcessor: Monitors token updates and generates alerts

Objective:
---------
1. Monitor token state changes after each block
2. Generate and publish alerts based on token behavior
3. Track alert history and prevent duplicate alerts
4. Provide configurable alert conditions

Architecture & Flow:
------------------
1. Token State Monitoring:
   - Subscribes to BlockLiveTokenProcessor updates
   - Receives list of updated tokens after each block
   - Tracks token state changes and metrics

2. Alert Generation Pipeline:
   - Analyzes token metrics and changes
   - Applies alert conditions and rules
   - Generates structured alert objects
   - Deduplicates and prioritizes alerts

3. Alert Publishing:
   - Publishes alerts to RabbitMQ exchange
   - Supports multiple alert types and severities
   - Handles backpressure and rate limiting
   - Ensures alert delivery and persistence

4. Alert Types:
   - Price manipulation alerts
   - Liquidity changes
   - Trading volume spikes
   - Ownership changes
   - Contract modifications
   - Scam indicators
"""

from typing import Dict, Set, List
from dataclasses import dataclass
from datetime import datetime
import asyncio

from eth_token.erc20_token.erc20_token import ERC20Token
from eth_token.utils.logger import get_logger


@dataclass
class TokenAlert:
    token_address: str
    alert_type: str
    severity: str
    description: str
    block_number: int
    timestamp: datetime
    metrics: Dict


class AlertProcessor:
    def __init__(self, rabbitmq_url: str = None, logger=None):
        if logger is None:
            self.logger = get_logger(name="tokens", log_folder="alert")
        self.processed_alerts: Set[str] = set()
        
    async def process_block_updates(self, block_number: int, updated_tokens: Dict[str, ERC20Token]):
        """Process token updates from a new block and generate alerts"""
        alerts = []
        
        for token_address, token in updated_tokens.items():
            try:
                # Check various alert conditions
                bribe_alerts = self._check_bribe_alerts(token)
                
                # Combine all alerts
                token_alerts = [
                    *bribe_alerts,
                ]
                
                # Deduplicate and filter alerts
                filtered_alerts = self._filter_alerts(token_alerts)
                alerts.extend(filtered_alerts)
                
            except Exception as e:
                self.logger.error(f"Error processing alerts for token {token_address}: {e}")
        
        # Publish alerts
        if alerts:
            await self.send_alert(alerts)
    
    def _check_bribe_alerts(self, token: ERC20Token) -> List[TokenAlert]:
        """Check for bribe alerts"""
        if token.bribe_amount > 0.5:
            return [
                TokenAlert(
                    token_address=token.contract_address,
                    alert_type="BRIBE_DETECTED",
                    severity="HIGH",
                    description=f"Bribe detected: {token.bribe_amount}",
                    block_number=token.bribe_block,
                    timestamp=datetime.now(),
                    metrics=token.metrics
                )
            ]
        return []
    
    def _check_price_alerts(self, token: ERC20Token) -> List[TokenAlert]:
        """Check for suspicious price movements"""
        alerts = []
        # Add price manipulation detection logic
        return alerts

    def _check_liquidity_alerts(self, token: ERC20Token) -> List[TokenAlert]:
        """Check for significant liquidity changes"""
        alerts = []
        # Add liquidity monitoring logic
        return alerts
    
    def _check_volume_alerts(self, token: ERC20Token) -> List[TokenAlert]:
        """Check for unusual trading volume"""
        alerts = []
        # Add volume analysis logic
        return alerts
    
    def _check_ownership_alerts(self, token: ERC20Token) -> List[TokenAlert]:
        """Check for ownership changes and contract modifications"""
        alerts = []
        # Add ownership monitoring logic
        return alerts
    
    def _check_scam_alerts(self, token: ERC20Token) -> List[TokenAlert]:
        """Check for potential scam indicators"""
        alerts = []
        if token.is_scam:
            alerts.append(
                TokenAlert(
                    token_address=token.contract_address,
                    alert_type="SCAM_DETECTED",
                    severity="HIGH",
                    description=f"Scam detected: {token.scam_label}",
                    block_number=token.scam_block,
                    timestamp=datetime.now(),
                    metrics=token.metrics
                )
            )
        return alerts
    
    def _filter_alerts(self, alerts: List[TokenAlert]) -> List[TokenAlert]:
        """Deduplicate and filter alerts"""
        filtered = []
        for alert in alerts:
            alert_key = f"{alert.token_address}:{alert.alert_type}:{alert.block_number}"
            if alert_key not in self.processed_alerts:
                self.processed_alerts.add(alert_key)
                filtered.append(alert)
        return filtered
    
    async def send_alert(self, alerts: List[TokenAlert]):
        """Send an alert"""
        try:
            for alert in alerts:
                self.logger.info(f"Sending alert: {alert}")
        except Exception as e:
            self.logger.error(f"Error sending alert: {e}")
        
    async def _publish_alerts(self, alerts: List[TokenAlert]):
        """Publish alerts to RabbitMQ"""
        try:
            for alert in alerts:
                await self.alert_publisher.publish_alert(alert)
        except Exception as e:
            self.logger.error(f"Error publishing alerts: {e}") 