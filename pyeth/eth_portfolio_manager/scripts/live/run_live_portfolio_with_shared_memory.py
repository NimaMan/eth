#!/usr/bin/env python3
"""
Run live portfolio manager with shared memory token cache.

This script demonstrates the integration of HybridLiveTokensCache
for ultra-low latency token data access from Rust components.
"""

import asyncio
import sys
import os
from pathlib import Path

# Add project root to path
project_root = Path(__file__).parent.parent.parent.parent
sys.path.insert(0, str(project_root))

from eth_portfolio_manager.backtesting.live_backtest_engine_with_pools import LiveBacktestEngineWithPools
from eth_portfolio_manager.strategy.buy_all import BuyAllStrategy
from eth_portfolio_manager.strategy.buy_scam import BuyScamStrategy
from eth_portfolio_manager.strategy.market_tracker import MarketTrackerStrategy
from eth_portfolio_manager.utils.logger import get_logger
from eth_token.token_manager.hybrid_live_tokens_cache import HybridLiveTokensCache

logger = get_logger(__name__, log_folder="live")


async def main():
    """Main entry point"""
    
    logger.info("=" * 80)
    logger.info("Starting Live Portfolio Manager with Shared Memory Cache")
    logger.info("=" * 80)
    
    # Configuration
    warmup_blocks = 100
    save_results = True
    add_pnl_to_db = True
    
    # Create strategies
    strategies = [
        MarketTrackerStrategy(),
        # BuyAllStrategy(),
        # BuyScamStrategy(),
    ]
    
    logger.info(f"Configured strategies: {[s.__class__.__name__ for s in strategies]}")
    logger.info(f"Warmup blocks: {warmup_blocks}")
    logger.info(f"Save results to DB: {save_results}")
    logger.info(f"Add PnL to DB: {add_pnl_to_db}")
    
    # Create custom token cache factory that returns HybridLiveTokensCache
    def create_hybrid_cache(max_size=2000, logger=None, add_pnl_to_db=False):
        """Factory function to create HybridLiveTokensCache"""
        logger.info("Creating HybridLiveTokensCache with shared memory support")
        cache = HybridLiveTokensCache(
            max_size=max_size,
            logger=logger,
            add_pnl_to_db=add_pnl_to_db,
            shm_size=50_000_000,  # 50MB
            enable_shared_memory=True
        )
        
        # Log shared memory stats
        stats = cache.get_shm_stats()
        logger.info(f"Shared memory initialized: {stats}")
        
        return cache
    
    # Create engine
    engine = LiveBacktestEngineWithPools(
        strategies=strategies,
        warmup_blocks=warmup_blocks,
        save_strategy_results=save_results,
        add_pnl_to_db=add_pnl_to_db,
        logger=logger,
        zmq_pub_endpoint="tcp://*:5557",
        zmq_rep_endpoint="tcp://*:5558",
        max_pools=2000,
        min_eth_threshold=0.01,
    )
    
    # Override the token cache creation in the block processor
    if hasattr(engine.live_token_processor, 'live_tokens_cache'):
        # Replace existing cache
        old_cache = engine.live_token_processor.live_tokens_cache
        new_cache = create_hybrid_cache(
            max_size=old_cache.max_size,
            logger=old_cache.logger,
            add_pnl_to_db=old_cache.add_pnl_to_db
        )
        engine.live_token_processor.live_tokens_cache = new_cache
        logger.info("Successfully replaced token cache with HybridLiveTokensCache")
    
    try:
        # Start the engine
        logger.info("Starting engine...")
        await engine.start()
        
        # Log shared memory stats periodically
        async def log_shm_stats():
            while True:
                await asyncio.sleep(60)  # Every minute
                if hasattr(engine.live_token_processor.live_tokens_cache, 'get_shm_stats'):
                    stats = engine.live_token_processor.live_tokens_cache.get_shm_stats()
                    logger.info(f"Shared memory stats: {stats}")
        
        # Start stats logging task
        stats_task = asyncio.create_task(log_shm_stats())
        
        # Run forever
        logger.info("Engine started successfully. Press Ctrl+C to stop.")
        await asyncio.Event().wait()
        
    except KeyboardInterrupt:
        logger.info("\nReceived interrupt signal, shutting down...")
    except Exception as e:
        logger.error(f"Error in main loop: {e}", exc_info=True)
    finally:
        # Stop the engine
        logger.info("Stopping engine...")
        await engine.stop()
        
        # Clean up shared memory
        if hasattr(engine.live_token_processor.live_tokens_cache, 'cleanup'):
            engine.live_token_processor.live_tokens_cache.cleanup()
            logger.info("Cleaned up shared memory resources")
        
        logger.info("Engine stopped. Goodbye!")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass