"""
Portfolio State Publisher

Objective:
---------
Publish portfolio state updates to RabbitMQ for external consumption
"""

import json
import aio_pika
from decimal import Decimal
from typing import Dict
from dataclasses import asdict


class PortfolioStatePublisher:
    def __init__(self, rabbitmq_url: str):
        self.rabbitmq_url = rabbitmq_url
        self.connection = None
        self.channel = None
        
    async def connect(self):
        """Establish RabbitMQ connection"""
        self.connection = await aio_pika.connect_robust(self.rabbitmq_url)
        self.channel = await self.connection.channel()
        await self.channel.declare_queue("portfolio_state")
        
    async def publish_state(self, portfolio_state: 'PortfolioState'):
        """Publish current portfolio state"""
        if not self.channel:
            await self.connect()
            
        # Convert portfolio state to serializable format
        state_data = {
            "total_value": str(portfolio_state.total_value),
            "total_profit_loss": str(portfolio_state.total_profit_loss),
            "positions": {
                addr: {
                    "quantity": str(pos.quantity),
                    "purchase_price": str(pos.purchase_price),
                    "current_price": str(pos.current_price),
                    "current_value": str(pos.current_value),
                    "profit_loss": str(pos.profit_loss),
                    "profit_loss_percentage": str(pos.profit_loss_percentage)
                }
                for addr, pos in portfolio_state.positions.items()
            }
        }
        
        message = aio_pika.Message(
            body=json.dumps(state_data).encode(),
            content_type="application/json"
        )
        
        await self.channel.default_exchange.publish(
            message,
            routing_key="portfolio_state"
        ) 