
"""
Token and Block Alert Processor Modules

Objective:
- Consume the updated live tokens from the block processor
- Process tokens using TokenAlertProcessor
- Publish alerts back to RabbitMQ
- Handle priorities and concurrent processing
"""

import time 
import asyncio
from typing import Dict
from eth_tokens_live.utils.logger import get_logger
from eth_tokens_live.alert.bribe_alert import BribeAlert
from eth_tokens_live.alert.green_actor_alert import GreenActorAlert
from eth_tokens_live.alert.scam_alert import ScamAlert
from eth_tokens_live.live_erc20_token.live_token import LiveERC20Token


ALERT_PROCESSORS = {
            'bribe': BribeAlert(),
            'scam': ScamAlert(),
            'green_actor': GreenActorAlert(),
        }


class TokenAlertProcessor:
    def __init__(self, rabbitmq_url: str = 'amqp://guest:guest@localhost/', logger=None):
        # Initialize all alert processors
        self.logger = logger or get_logger(name="tokens_manager", log_folder="tokens_live")
        self.rabbitmq_url = rabbitmq_url
        self.alert_processors = ALERT_PROCESSORS

    async def process_single_alert(self, alert_type: str, processor, live_erc20_token: LiveERC20Token):
        """Process a single alert type asynchronously"""
        try:
            alerts = await processor.process_token(live_erc20_token)
            if alerts:
                self.logger.debug(f"Generated {alert_type} alerts for token {live_erc20_token.contract_address}: {len(alerts)}")
            return alerts
        except Exception as e:
            self.logger.error(f"{__name__}: Error processing {alert_type} alert for token {live_erc20_token.contract_address}: {str(e)}")
            return []

    async def process_token(self, live_erc20_token: LiveERC20Token):
        """
        Process a token through all alert processors concurrently
        
        Args:
            live_erc20_token: LiveERC20Token to process
            
        Returns:
            List of alerts generated from all processors
        """
        try:
            # Create tasks for all alert processors
            alert_tasks = [
                self.process_single_alert(alert_type, processor, live_erc20_token)
                for alert_type, processor in self.alert_processors.items()
            ]
            
            # Execute all alert processing concurrently
            results = await asyncio.gather(*alert_tasks, return_exceptions=True)
            
            # Aggregate alerts, filtering out errors and empty results
            all_alerts = []
            for result in results:
                if isinstance(result, list):
                    all_alerts.extend(result)
                elif isinstance(result, Exception):
                    self.logger.error(f"{__name__}: Alert processing error for token {live_erc20_token.contract_address}: {str(result)}")
            return all_alerts
            
        except Exception as e:
            self.logger.error(f"{__name__}: Critical error in alert processing for token {live_erc20_token.contract_address}: {str(e)}")
            return []


class UpdateLiveTokenAlertProcessor:
    def __init__(self, rabbitmq_url: str = 'amqp://guest:guest@localhost/', logger=None):
        self.logger = logger or get_logger(name="tokens_manager", log_folder="tokens_live")
        self.token_alert_processor = TokenAlertProcessor(rabbitmq_url=rabbitmq_url, logger=self.logger)

    async def process_block_token_updates(self, updated_tokens: Dict[str, LiveERC20Token], block_number: int):
        """Process list of token dictionaries concurrently"""
        try:
            start_time = time.time()
            
            # Process transactions concurrently
            token_tasks = [
                self.token_alert_processor.process_token(token)
                for token in updated_tokens.values()
            ]
            
            # Wait for all transaction processing to complete
            alerts_nested = await asyncio.gather(*token_tasks, return_exceptions=True)
            
            # Flatten and filter alerts
            valid_alerts = []
            for alert_list in alerts_nested:
                if isinstance(alert_list, Exception):
                    continue
                if isinstance(alert_list, list):
                    valid_alerts.extend([
                        alert for alert in alert_list 
                        if alert and not isinstance(alert, (Exception, list))
                    ])
            
            self.logger.info(f"Processed block {block_number} in {time.time() - start_time:.2f} seconds with {len(valid_alerts)} alerts")
            return valid_alerts
        
        except Exception as e:
            self.logger.error(f"{__name__}: Error processing transactions: {e}", exc_info=True)
            return []
