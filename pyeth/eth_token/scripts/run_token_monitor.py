"""
Token Manager Script

Objective:
---------
1. Initialize and run the token manager service
2. Handle proper startup with historical processing
3. Manage graceful shutdown
4. Monitor token creation and updates
"""

import asyncio
import signal
import sys
from eth_tokens_live.token_manager.live_token_manager import LiveTokenManager
from eth_tokens_live.utils.logger import get_logger


logger = get_logger(name="token_manager", log_folder="tokens_live")


class TokenManagerService:
    def __init__(self, warmup_blocks: int = 1000):
        self.token_manager = LiveTokenManager(
            logger=logger,
            warmup_blocks=warmup_blocks
        )
        self._shutdown_event = asyncio.Event()
        self._is_shutting_down = False
        self._main_task = None  # Add this to track the main task

    async def start(self):
        """Start the token manager service"""
        def handle_signal():
            if not self._is_shutting_down:
                logger.info("Received shutdown signal")
                # Create task but don't await it here
                asyncio.create_task(self.shutdown())
        
        # Setup signal handlers
        for sig in (signal.SIGTERM, signal.SIGINT):
            asyncio.get_event_loop().add_signal_handler(
                sig,
                handle_signal
            )

        try:
            logger.info("Starting token manager service")
            # Start token manager (this includes historical processing)
            self._main_task = asyncio.create_task(self.token_manager.start())
            
            # Wait for either shutdown event or main task completion
            done, pending = await asyncio.wait(
                [self._shutdown_event.wait(), self._main_task],
                return_when=asyncio.FIRST_COMPLETED
            )
            
            # Cancel any pending tasks
            for task in pending:
                task.cancel()
                try:
                    await task
                except asyncio.CancelledError:
                    pass
            
        except Exception as e:
            logger.error(f"Error in token manager service: {e}")
            raise
        finally:
            await self.cleanup()

    async def shutdown(self):
        """Handle graceful shutdown"""
        if self._is_shutting_down:
            return
            
        self._is_shutting_down = True
        logger.info("Initiating service shutdown...")
        
        # Signal shutdown
        self._shutdown_event.set()
        
        # Stop token manager
        try:
            await self.token_manager.stop()
        except Exception as e:
            logger.error(f"Error stopping token manager: {e}")

    async def cleanup(self):
        """Cleanup resources"""
        if not self._is_shutting_down:
            await self.shutdown()


def main(warmup_blocks: int = 1000):
    # Initialize service with 1000 blocks warm-up
    service = TokenManagerService(warmup_blocks=warmup_blocks)
    
    try:
        # Run with proper signal handling
        asyncio.run(service.start())
    except KeyboardInterrupt:
        logger.info("Received keyboard interrupt")
    except Exception as e:
        logger.error(f"Error running token manager: {e}", exc_info=True)
        sys.exit(1)


if __name__ == "__main__":
    main(warmup_blocks=1000) 