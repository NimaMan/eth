"""
Live Portfolio Monitor Script with Mempool Monitoring

Objective:
---------
1. Run real-time portfolio position tracking with multiple strategies
2. Connect to live blockchain data stream via RabbitMQ
3. Apply same strategies used in backtesting to live data
4. Monitor mempool for potential scam transactions
5. Log suspicious activity before it's confirmed
6. Persist results for dashboard consumption
"""

import signal
import time
import asyncio
import sys
from web3 import Web3

from eth_portfolio_manager.backtesting.live_backtest_engine_with_mempool import LiveBacktestEngineWithMempool
from eth_portfolio_manager.strategy import BuyAll, BuyScamStrategy, MarketTracker
from eth_portfolio_manager.utils.logger import get_logger


logger = get_logger(name="portfolio_live_mempool", log_folder="live")


class LivePortfolioConfig:
    def __init__(self, initial_balance: float = 1.0):
        self.initial_balance = initial_balance
        # Use the same strategies as in backtesting for consistency
        self.strategies = {
            "MarketTracker": MarketTracker(),
            }


class LivePortfolioServiceWithMempool:
    def __init__(self, 
                 w3,
                 config, 
                 logger=None, 
                 warmup_blocks=10000, 
                 poll_interval=0.5,
                 eth_threshold=0.05):
        self.logger = logger
        self.engine = LiveBacktestEngineWithMempool(
            w3=w3,
            config=config,
            logger=self.logger,
            warmup_blocks=warmup_blocks,
            poll_interval=poll_interval,
            eth_threshold=eth_threshold
        )
        self._shutdown_event = asyncio.Event()
        self._is_shutting_down = False
        self._main_task = None

    async def start(self):
        """Start the portfolio service with mempool monitoring"""
        def handle_signal():
            if not self._is_shutting_down:
                self.logger.info("Received shutdown signal")
                # Create task but don't await it here
                asyncio.create_task(self.shutdown())
        
        # Setup signal handlers
        for sig in (signal.SIGTERM, signal.SIGINT):
            asyncio.get_event_loop().add_signal_handler(
                sig,
                handle_signal
            )

        try:
            self.logger.info("Starting portfolio service with mempool monitoring")
            # Start execution engine
            await self.engine.start()  # This returns quickly
            
            # Keep the service running until shutdown
            self.logger.info("Service startup complete - running indefinitely until shutdown")
            await self._shutdown_event.wait()  # Just wait for shutdown event
        
        except Exception as e:
            self.logger.error(f"Error in portfolio service: {e}")
            raise
        finally:
            await self.cleanup()

    async def shutdown(self):
        """Handle graceful shutdown"""
        if self._is_shutting_down:
            return
            
        self._is_shutting_down = True
        self.logger.info("Initiating service shutdown...")
        
        # Signal shutdown
        self._shutdown_event.set()
        
        # Stop execution engine
        try:
            await self.engine.stop()
        except Exception as e:
            self.logger.error(f"Error stopping execution engine: {e}")

    async def cleanup(self):
        """Cleanup resources"""
        if not self._is_shutting_down:
            await self.shutdown()


async def run_live_portfolio_with_mempool(warmup_blocks=10000, poll_interval=0.5, eth_threshold=0.05):
    """Run the live portfolio monitoring service with mempool monitoring"""
    start_time = time.time()
    
    # Log startup information
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    latest_block = w3.eth.get_block_number()
    warmup_start_block = latest_block - warmup_blocks
    
    logger.info(f"Starting live portfolio monitoring with mempool at block {latest_block}")
    logger.info(f"Warming up from block {warmup_start_block}")
    logger.info(f"Mempool poll interval: {poll_interval}s, ETH threshold: {eth_threshold}")
    
    # Create configuration with the same strategies as backtesting
    config = LivePortfolioConfig(initial_balance=1.0)
    
    # Initialize service with mempool monitoring
    service = LivePortfolioServiceWithMempool(
        w3=w3,
        config=config,
        logger=logger,
        warmup_blocks=warmup_blocks,
        poll_interval=poll_interval,
        eth_threshold=eth_threshold
    )
    
    # Start service
    await service.start()
    
    duration = (time.time() - start_time) / 60
    logger.info(f"Live portfolio service ran for {duration:.2f} minutes")


if __name__ == "__main__":
    try:
        # Run with proper signal handling
        asyncio.run(run_live_portfolio_with_mempool(
            warmup_blocks=1200,
            poll_interval=0.5,
            eth_threshold=0.4 
        ))
    except KeyboardInterrupt:
        logger.info("Received keyboard interrupt")
    except Exception as e:
        logger.error(f"Error running portfolio service: {e}", exc_info=True)
        sys.exit(1) 