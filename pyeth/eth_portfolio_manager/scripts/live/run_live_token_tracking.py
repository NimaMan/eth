"""
Live Portfolio Monitor with Pool Level Tracking and Live Trading Database Integration

Objective:
---------
1. Run real-time portfolio position tracking with multiple strategies
2. Extract and track pool ETH reserve levels from token updates
3. Publish pool levels via ZeroMQ to the Rust mempool processor
4. Use live_trading_db for enhanced position tracking and signal publishing
5. Enable scam detection in pending transactions before they're confirmed
6. Maintain bidirectional communication between Python and Rust components
7. Persist strategy results and positions for dashboard consumption

This script implements the enhanced version with live_trading_db:
- Python handles confirmed block processing and pool level tracking
- Rust handles mempool monitoring and scam detection
- ZeroMQ provides the communication bridge between components
- live_trading_db provides comprehensive position and signal tracking
- Signal publishing to eth_kartal for trade execution
"""

import signal
import time
import asyncio
import os
import sys
import atexit
import traceback
from web3 import Web3

from eth_portfolio_manager.live_trading.live_token_tracker import LiveTokenTracker
from eth_portfolio_manager.live_trading import LiveTradingIntegrator
from eth_portfolio_manager.strategy import MarketTracker, WalletTrackerStrategy
from eth_portfolio_manager.utils.logger import get_logger


logger = get_logger(name="live_token_tracking", log_folder="live_trading")


class LivePortfolioConfig:
    def __init__(self, initial_balance: float = 1.0):
        self.initial_balance = initial_balance
        
        # Create wallet tracker strategy for the specified wallet
        wallet_strategy = WalletTrackerStrategy()
        wallet_strategy.config.wallet_address = "0x9f056A8d8F9127837B4c8005FA91BCeE6072ba81"
        wallet_strategy.config.position_size_eth = 0.01
        wallet_strategy.config.max_positions = 10
        
        # Use the same strategies as in backtesting for consistency
        self.strategies = {
            "MarketTracker": MarketTracker(),
            "WalletTracker": wallet_strategy
        }


class LivePortfolioServiceWithPoolSharing:
    """Enhanced live portfolio service with live_trading_db integration."""
    
    def __init__(self, 
                 config, 
                 logger=None, 
                 warmup_blocks=10000,
                 save_strategy_results=False,
                 add_pnl_to_db=False,
                 add_status_to_db=False,
                 min_eth_threshold=0.05):
        self.logger = logger
        self.config = config
        
        # Create the LiveTokenTracker (existing working system)
        self.engine = LiveTokenTracker(
            config=config,
            logger=self.logger,
            warmup_blocks=warmup_blocks,
            save_strategy_results=save_strategy_results,
            add_pnl_to_db=add_pnl_to_db,
            add_status_to_db=add_status_to_db,
            min_eth_threshold=min_eth_threshold
        )
        
        # Integrate with live_trading_db using the adapter
        self.strategies = list(config.strategies.values())
        self.adapter = None
        if save_strategy_results:
            self.adapter = LiveTradingIntegrator.replace_results_writer(
                self.engine, self.strategies
            )
        
        if self.adapter:
            self.logger.info("✅ Successfully integrated LiveTokenTracker with live_trading_db")
        else:
            self.logger.warning("⚠️ Failed to integrate live_trading_db, using original system")
        
        self._shutdown_event = asyncio.Event()
        self._is_shutting_down = False

    async def start(self):
        """Start the portfolio service with pool level tracking"""
        def handle_signal(sig_name: str, sig_no: int):
            if not self._is_shutting_down:
                self.logger.warning(
                    f"Received shutdown signal: {sig_name} ({sig_no}) | pid={os.getpid()}"
                )
                # Create task but don't await it here
                asyncio.create_task(self.shutdown())
        
        # Setup signal handlers
        loop = asyncio.get_event_loop()
        for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGQUIT):
            # Bind current sig into closure for logging
            loop.add_signal_handler(sig, lambda s=sig: handle_signal(signal.Signals(s).name, int(s)))

        try:
            self.logger.info("Starting portfolio service with live_trading_db integration")
            
            # Start the LiveTokenTracker (this handles all the pool publishing, etc.)
            await self.engine.start()
            
            # Watch engine core tasks; if any exits, shut down service
            async def _watch_engine():
                tasks = [
                    getattr(self.engine, 'token_processor_task', None),
                    getattr(self.engine, '_main_token_processing_task', None),
                ]
                tasks = [t for t in tasks if t]
                if not tasks:
                    return
                done, _ = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
                for t in done:
                    try:
                        exc = t.exception()
                    except asyncio.CancelledError:
                        self.logger.info("Engine task cancelled (likely due to shutdown).")
                        return
                    except Exception as e:
                        self.logger.error(f"Error retrieving engine task exception: {e}", exc_info=True)
                        exc = None
                    if exc:
                        self.logger.error(f"Engine task exited with error: {exc}", exc_info=True)
                    else:
                        self.logger.warning("Engine task exited unexpectedly without error.")
                await self.shutdown()

            asyncio.create_task(_watch_engine())
            
            # Log integration status
            if self.adapter:
                self.logger.info("🚀 Live trading database integration active")
                self.logger.info("📊 Position tracking enhanced with live_trading_db")
                self.logger.info("📡 Signal publishing to eth_kartal enabled")
            
            # Log ZMQ endpoints from the live tracker
            pub_endpoint = getattr(self.engine.token_tracking_publisher, 'pub_endpoint', 'Unknown')
            rep_endpoint = getattr(self.engine.token_tracking_publisher, 'rep_endpoint', 'Unknown')
            self.logger.info(f"ZeroMQ PUB socket bound to {pub_endpoint}")
            self.logger.info(f"ZeroMQ REP socket bound to {rep_endpoint}")
            self.logger.info("Waiting for Rust mempool processor to connect...")
            
            # Keep the service running until shutdown
            self.logger.info("Service startup complete - running indefinitely until shutdown")
            await self._shutdown_event.wait()
        
        except asyncio.CancelledError as e:
            # Task was cancelled (propagate) but log it first with stack for postmortem
            self.logger.warning("Service task cancelled", exc_info=True)
            raise
        except Exception as e:
            self.logger.error(f"Error in portfolio service: {e}", exc_info=True)
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
        
        # Stop the live tracker
        try:
            await self.engine.stop()
        except Exception as e:
            self.logger.error(f"Error stopping live tracker: {e}", exc_info=True)
        
        # Shutdown the adapter
        if self.adapter:
            try:
                await self.adapter.shutdown()
            except Exception as e:
                self.logger.error(f"Error shutting down live trading adapter: {e}", exc_info=True)

    async def cleanup(self):
        """Cleanup resources"""
        if not self._is_shutting_down:
            await self.shutdown()


async def run_live_portfolio_with_pool_sharing(
    warmup_blocks=10000, 
    save_strategy_results=False,
    add_pnl_to_db=False,
    add_status_to_db=False,
    min_eth_threshold=0.01
):
    """Run the live portfolio monitoring service with pool level tracking"""
    start_time = time.time()
    
    # Connect to Ethereum node for information purposes only
    eth_node_url = os.environ.get("ETH_NODE_URL", "http://127.0.0.1:8545")
    w3 = Web3(Web3.HTTPProvider(eth_node_url))
    
    try:
        latest_block = w3.eth.block_number
        warmup_start_block = latest_block - warmup_blocks
        chain_id = w3.eth.chain_id
        
        logger.info(f"Connected to Ethereum node at {eth_node_url}")
        logger.info(f"Chain ID: {chain_id}, Latest block: {latest_block}")
        logger.info(f"Starting live portfolio monitoring with live_trading_db at block {latest_block}")
        logger.info(f"Warming up from block {warmup_start_block}")
        logger.info(f"Min ETH threshold: {min_eth_threshold}")
        logger.info(f"PnL writing to database: {'ENABLED' if add_pnl_to_db else 'DISABLED'}")
    except Exception as e:
        logger.warning(f"Could not fetch blockchain information: {e}")
        logger.info("Continuing anyway as the LiveBlockTokenProcessor will handle connections")
    
    # Create configuration with the same strategies as backtesting
    config = LivePortfolioConfig(initial_balance=1.0)
    
    # Initialize service with live_trading_db integration
    service = LivePortfolioServiceWithPoolSharing(
        config=config,
        logger=logger,
        warmup_blocks=warmup_blocks,
        save_strategy_results=save_strategy_results,
        add_pnl_to_db=add_pnl_to_db,
        add_status_to_db=add_status_to_db,
        min_eth_threshold=min_eth_threshold
    )
    
    # Start service
    await service.start()
    
    duration = (time.time() - start_time) / 60
    logger.info(f"Live portfolio service ran for {duration:.2f} minutes")


if __name__ == "__main__":
    warmup_blocks = 10000  # Number of blocks to warm up on startup  
    save_strategy_results = True  # Enable to use live_trading_db
    add_pnl_to_db = False  # Disable PnL tracking to avoid writer errors
    min_eth_threshold = 0.01  # Minimum ETH reserve (0.01 ETH)
    
    # Ensure unexpected exceptions are logged
    def _excepthook(exc_type, exc, tb):
        logger.error("Uncaught exception", exc_info=(exc_type, exc, tb))
    sys.excepthook = _excepthook

    # Log interpreter exit reason
    @atexit.register
    def _on_exit():
        # If normal shutdown path executed, a shutdown log already exists
        logger.info("Python process exiting (atexit). If unplanned, check logs above for exception or signal.")

    try:
        asyncio.run(run_live_portfolio_with_pool_sharing(
                warmup_blocks=warmup_blocks,
                save_strategy_results=save_strategy_results,
                add_pnl_to_db=add_pnl_to_db,
                min_eth_threshold=min_eth_threshold
        ))
    except KeyboardInterrupt:
        logger.info("Received keyboard interrupt (SIGINT)")
    except asyncio.CancelledError:
        logger.warning("Main asyncio.run cancelled", exc_info=True)
    except Exception as e:
        logger.error(f"Error running portfolio service: {e}", exc_info=True)
        sys.exit(1)
    
