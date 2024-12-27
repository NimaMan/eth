"""
Token Manager Script

Objective:
---------
1. Subscribe to existing block processor messages
2. Track and update token states
3. Handle graceful shutdown
4. Monitor token creation and updates
"""

import asyncio
import signal
from typing import Optional
import sys
import os

from eth_tokens_live.token_manager.live_token_manager import LiveTokenManager
from eth_tokens_live.utils.logger import get_logger


logger = get_logger(name="token_manager", log_folder="tokens_live")


class TokenManagerService:
    def __init__(
        self,
        rabbitmq_url: str = "amqp://guest:guest@localhost/",
        max_queue_size: int = 1000
    ):
        self.token_manager = LiveTokenManager(rabbitmq_url=rabbitmq_url, logger=logger)
        self.is_running = False
        self._is_shutting_down = False
        self._shutdown_event = asyncio.Event()

    async def start(self):
        """Start the token manager service"""
        self.is_running = True
        
        def handle_signal():
            if not self._is_shutting_down:
                asyncio.create_task(self.shutdown())
        
        # Setup signal handlers
        for sig in (signal.SIGTERM, signal.SIGINT):
            asyncio.get_event_loop().add_signal_handler(
                sig,
                handle_signal
            )

        try:
            # Start token manager
            await self.token_manager.start()
            
            # Wait for shutdown signal
            await self._shutdown_event.wait()
            
        except KeyboardInterrupt:
            await self.shutdown()
        finally:
            await self.cleanup()

    async def shutdown(self):
        """Handle graceful shutdown"""
        if self._is_shutting_down:
            return
            
        self._is_shutting_down = True
        logger.info("Shutting down...")
        
        self.is_running = False
        self._shutdown_event.set()
        
        # Signal token manager to stop
        await self.token_manager.stop()

    async def cleanup(self):
        """Cleanup resources"""
        if not self._is_shutting_down:
            await self.shutdown()

def main():
    # Configuration
    config = {
        'rabbitmq_url': os.getenv('RABBITMQ_URL', 'amqp://guest:guest@localhost/'),
        'max_queue_size': int(os.getenv('MAX_QUEUE_SIZE', '1000'))
    }

    # Initialize service
    service = TokenManagerService(**config)

    # Run the service
    try:
        asyncio.run(service.start())
    except KeyboardInterrupt:
        logger.info("Shutting down...")
    except Exception as e:
        logger.error(f"Error running token manager: {e}", exc_info=True)
        sys.exit(1)


if __name__ == "__main__":
    main() 