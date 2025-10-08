import asyncio  
from typing import Dict, Optional
from web3 import Web3

from eth_data.blockchain.mempool_processor import MempoolProcessor
from eth_token.utils.logger import get_logger
from .tax_manipulation_subscriber import TaxManipulationSubscriber


class TokenMempoolTracker:
    
    def __init__(self, token_processor, logger=None, poll_interval: float = 0.5, 
                 token_cache=None, enable_tax_monitoring: bool = True):
        self.logger = logger
        self.token_processor = token_processor
        
        # Get Web3 instance from token processor if available
        self.web3 = getattr(token_processor, 'w3', None) or Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        
        # Create mempool monitor
        self.mempool_monitor = MempoolProcessor(
            w3=self.web3,
            logger=self.logger,
            poll_interval=poll_interval
        )
        
        # Tax manipulation monitoring
        self.tax_manipulation_subscriber: Optional[TaxManipulationSubscriber] = None
        if enable_tax_monitoring and token_cache:
            self.tax_manipulation_subscriber = TaxManipulationSubscriber(
                token_cache=token_cache,
                logger=self.logger
            )
        
        # For tracking confirmations
        self._confirmation_task = None
        self._shutdown_event = asyncio.Event()
    
    async def start(self):
        """Start both token processing and mempool monitoring."""
        # Start the token processor if it has a start method
        if hasattr(self.token_processor, 'start') and callable(self.token_processor.start):
            await self.token_processor.start()
        
        # Set up callback for token-related transactions
        self.mempool_monitor.on_token_transaction = self._on_token_transaction
        
        # Start mempool monitor
        await self.mempool_monitor.start()
        
        # Start tax manipulation monitoring if enabled
        if self.tax_manipulation_subscriber:
            asyncio.create_task(self.tax_manipulation_subscriber.start())
            self.logger.info("Started tax manipulation monitoring")
        
        # Start confirmation tracking
        self._confirmation_task = asyncio.create_task(self._track_confirmations())
        
        self.logger.info("Started token mempool tracker")
    
    async def stop(self):
        """Stop mempool monitoring and cleanup resources."""
        self._shutdown_event.set()
        
        # Stop confirmation tracking
        if self._confirmation_task and not self._confirmation_task.done():
            self._confirmation_task.cancel()
            try:
                await self._confirmation_task
            except asyncio.CancelledError:
                pass
        
        # Stop tax manipulation monitoring
        if self.tax_manipulation_subscriber:
            await self.tax_manipulation_subscriber.stop()
            self.logger.info("Stopped tax manipulation monitoring")
        
        # Stop mempool monitor
        if hasattr(self.mempool_monitor, 'stop'):
            await self.mempool_monitor.stop()
        
        # Stop token processor if it has a stop method
        if hasattr(self.token_processor, 'stop') and callable(self.token_processor.stop):
            await self.token_processor.stop()
        
        self.logger.info("Stopped token mempool tracker")
    
    