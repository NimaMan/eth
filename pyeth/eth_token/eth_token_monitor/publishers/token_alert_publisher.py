"""
TokenAlertPublisher: Publishes token-related alerts to RabbitMQ

Objective:
---------
1. Detect and publish token-related alerts
2. Handle different types of alerts (bribe, scam, etc.)
3. Provide extensible alert system
4. Ensure reliable alert delivery

Alert Types:
----------
1. Bribe Alerts:
   - Track ETH transfers above threshold
   
Future Alert Types (TODO):
-----------------------
- Scam Prediction Detection
- Orcas
- Whales 

"""

import aio_pika
from typing import Dict, Any
from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token
import orjson

from eth_token_monitor.utils.logger import get_logger


logger = get_logger(name="token_alerts", log_folder="tokens_live")


class TokenAlertPublisher:
    def __init__(
        self, 
        rabbitmq_url: str,
        exchange_name: str = "token_alerts",
        bribe_threshold: float = 0.5  # 0.5 ETH default threshold
    ):
        self.rabbitmq_url = rabbitmq_url
        self.exchange_name = exchange_name
        self.bribe_threshold = bribe_threshold
        
        self.connection = None
        self.channel = None
        self.exchange = None
        
    async def connect(self):
        """Establish RabbitMQ connection"""
        try:
            self.connection = await aio_pika.connect_robust(self.rabbitmq_url)
            self.channel = await self.connection.channel()
            
            # Declare exchange
            self.exchange = await self.channel.declare_exchange(
                self.exchange_name,
                aio_pika.ExchangeType.TOPIC,
                durable=True
            )
            
            logger.info(f"Connected to RabbitMQ, exchange: {self.exchange_name}")
            
        except Exception as e:
            logger.error(f"Error connecting to RabbitMQ: {e}")
            raise
            
    async def disconnect(self):
        """Close RabbitMQ connection"""
        try:
            if self.connection and not self.connection.is_closed:
                await self.connection.close()
                self.exchange = None
                self.channel = None
                self.connection = None
        except Exception as e:
            logger.error(f"Error disconnecting from RabbitMQ: {e}")

    def check_bribe_alert(self, live_token: LiveERC20Token) -> Dict[str, Any]:
        """Check if transaction contains bribe patterns"""
        bribe_amount = live_token.total_bribe_amount        
        if bribe_amount >= self.bribe_threshold:
            return {
                'alert_type': 'bribe_detected',
                'severity': 'high',
                'bribe_amount': float(bribe_amount),
                'from_address': live_token.creator,
                'to_address': live_token.creator,
                'transaction_hash': live_token.creation_txn_hash,
                'block_number': live_token.creation_block_number
            }
        return None

    async def publish_alert(self, alert_data: Dict):
        """Publish alert to RabbitMQ"""
        try:
            if not self.exchange:
                logger.error("No RabbitMQ connection available")
                return False
                
            routing_key = f"alerts.{alert_data['alert_type']}"
            
            message = aio_pika.Message(
                body=orjson.dumps(alert_data),
                delivery_mode=aio_pika.DeliveryMode.PERSISTENT
            )
            await self.exchange.publish(
                message,
                routing_key=routing_key
            )            
            logger.info(f"Published {alert_data['alert_type']} alert for token: {alert_data['token_address']}")
            return True
        except Exception as e:
            logger.error(f"Error publishing alert: {e}")
            return False

    async def process_token(self, live_token: LiveERC20Token):
        """Process transaction for all alert types"""
        try:
            # Check for bribe alert
            bribe_alert = self.check_bribe_alert(live_token)
            if bribe_alert:
                await self.publish_alert(bribe_alert)
                
            # Future alert types will be added here
        except Exception as e:
            logger.error(f"Error processing transaction for alerts: {e}")
