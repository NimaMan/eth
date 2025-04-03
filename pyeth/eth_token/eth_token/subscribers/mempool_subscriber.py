import asyncio  
from typing import Dict
from web3 import Web3

from eth_block_processor.blockchain.mempool_processor import MempoolProcessor
from eth_token.utils.logger import get_logger


class TokenMempoolTracker:
    
    def __init__(self, token_processor, logger=None, poll_interval: float = 0.5):
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
        
        # Start confirmation tracking
        self._confirmation_task = asyncio.create_task(self._track_confirmations())
        
        self.logger.info("Started token mempool tracker")
    
    