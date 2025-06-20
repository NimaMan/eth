"""
Live Portfolio Monitor with Pool Level Tracking and Publishing

Objective:
---------
1. Run real-time portfolio position tracking with multiple strategies
2. Extract and track pool ETH reserve levels from token updates
3. Publish pool levels via ZeroMQ to the Rust mempool processor
4. Enable scam detection in pending transactions before they're confirmed
5. Maintain a bidirectional communication channel between Python and Rust components
6. Persist strategy results and positions for dashboard consumption

This script implements Phase 2 of the Ethereum Mempool Processor architecture:
- Python handles confirmed block processing and pool level tracking
- Rust handles mempool monitoring and scam detection
- ZeroMQ provides the communication bridge between components
"""

import signal
import time
import asyncio
import sys
import os
from web3 import Web3

from eth_portfolio_manager.backtesting.live_backtest_engine_with_pools import LiveBacktestEngineWithPools
from eth_portfolio_manager.strategy import MarketTracker
from eth_portfolio_manager.utils.logger import get_logger


logger = get_logger(name="live_portfolio", log_folder="live")


class LivePortfolioConfig:
    def __init__(self, initial_balance: float = 1.0):
        self.initial_balance = initial_balance
        # Use the same strategies as in backtesting for consistency
        self.strategies = {
            "MarketTracker": MarketTracker(),
        }


class LivePortfolioServiceWithPools:
    def __init__(self, 
                 config, 
                 logger=None, 
                 warmup_blocks=10000,
                 save_strategy_results=True,
                 add_pnl_to_db=True,
                 zmq_pub_endpoint="tcp://*:5557",
                 zmq_rep_endpoint="tcp://*:5558",
                 max_pools=2000,
                 min_eth_threshold=0.01):
        self.logger = logger
        self.engine = LiveBacktestEngineWithPools(
            config=config,
            logger=self.logger,
            warmup_blocks=warmup_blocks,
            save_strategy_results=save_strategy_results,
            add_pnl_to_db=add_pnl_to_db,
            zmq_pub_endpoint=zmq_pub_endpoint,
            zmq_rep_endpoint=zmq_rep_endpoint,
            max_pools=max_pools,
            min_eth_threshold=min_eth_threshold
        )
        self._shutdown_event = asyncio.Event()
        self._is_shutting_down = False
        self._main_task = None

    async def start(self):
        """Start the portfolio service with pool level tracking"""
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
            self.logger.info("Starting portfolio service with pool level tracking")
            
            # Start execution engine
            await self.engine.start()  # This returns quickly
            
            # Log ZMQ endpoints
            self.logger.info(f"ZeroMQ PUB socket bound to {self.engine.pool_level_publisher.pub_endpoint}")
            self.logger.info(f"ZeroMQ REP socket bound to {self.engine.pool_level_publisher.rep_endpoint}")
            self.logger.info("Waiting for Rust mempool processor to connect...")
            
            # Keep the service running until shutdown
            self.logger.info("Service startup complete - running indefinitely until shutdown")
            await self._shutdown_event.wait()  # Just wait for shutdown event
        
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
        
        # Stop execution engine
        try:
            await self.engine.stop()
        except Exception as e:
            self.logger.error(f"Error stopping execution engine: {e}", exc_info=True)

    async def cleanup(self):
        """Cleanup resources"""
        if not self._is_shutting_down:
            await self.shutdown()


async def run_live_portfolio_with_pools(
    warmup_blocks=10000, 
    save_strategy_results=True,
    add_pnl_to_db=True,
    zmq_pub_endpoint="tcp://*:5557",
    zmq_rep_endpoint="tcp://*:5558",
    max_pools=2000,
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
        logger.info(f"Starting live portfolio monitoring with pool tracking at block {latest_block}")
        logger.info(f"Warming up from block {warmup_start_block}")
        logger.info(f"ZMQ PUB endpoint: {zmq_pub_endpoint}")
        logger.info(f"ZMQ REP endpoint: {zmq_rep_endpoint}")
        logger.info(f"Max pools: {max_pools}, Min ETH threshold: {min_eth_threshold}")
    except Exception as e:
        logger.warning(f"Could not fetch blockchain information: {e}")
        logger.info("Continuing anyway as the LiveBlockTokenProcessor will handle connections")
    
    # Create configuration with the same strategies as backtesting
    config = LivePortfolioConfig(initial_balance=1.0)
    
    # Initialize service with pool level tracking
    service = LivePortfolioServiceWithPools(
        config=config,
        logger=logger,
        warmup_blocks=warmup_blocks,
        save_strategy_results=save_strategy_results,
        add_pnl_to_db=add_pnl_to_db,
        zmq_pub_endpoint=zmq_pub_endpoint,
        zmq_rep_endpoint=zmq_rep_endpoint,
        max_pools=max_pools,
        min_eth_threshold=min_eth_threshold
    )
    
    # Start service
    await service.start()
    
    duration = (time.time() - start_time) / 60
    logger.info(f"Live portfolio service ran for {duration:.2f} minutes")


if __name__ == "__main__":
    warmup_blocks = 1200
    save_strategy_results = False
    add_pnl_to_db = False
    zmq_pub_endpoint = "tcp://*:5557"
    zmq_rep_endpoint = "tcp://*:5558"
        
    max_pools = 2000  # Maximum number of pools to track
    min_eth_threshold = 0.01  # Minimum ETH reserve (0.01 ETH)
    
    asyncio.run(run_live_portfolio_with_pools(
            warmup_blocks=warmup_blocks,
            save_strategy_results=save_strategy_results,
            add_pnl_to_db=add_pnl_to_db,
            zmq_pub_endpoint=zmq_pub_endpoint,
            zmq_rep_endpoint=zmq_rep_endpoint,
            max_pools=max_pools,
            min_eth_threshold=min_eth_threshold
    ))
    