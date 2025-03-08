"""
Portfolio Manager Runner

Objective:
---------
1. Initialize and run portfolio manager
2. Handle graceful shutdown
3. Provide CLI options for configuration
"""

import asyncio

from eth_portfolio_manager.strategy.buy_all import BuyAll
from eth_portfolio_manager.strategy.buy_scam import BuyScamStrategy
from eth_portfolio_manager.live.live_portfolio_manager import LivePortfolioManager
from eth_portfolio_manager.utils.logger import get_logger


logger = get_logger(name="portfolio_manager", log_folder="portfolio_manager")


BUY_EVERYTHING_STRATEGY = BuyAll
BUY_SCAM_STRATEGY = BuyScamStrategy



class LivePortfolioConfig:
    initial_balance: float = 1.0  # ETH
    strategies = [BUY_EVERYTHING_STRATEGY, BUY_SCAM_STRATEGY]


async def run_portfolio_manager(config: LivePortfolioConfig):
    """Run the portfolio manager"""
    try:
        portfolio_manager = LivePortfolioManager(config=config, logger=logger, warmup_blocks=1000)
        await portfolio_manager.start()
        
    except KeyboardInterrupt:
        logger.info("Shutdown requested...")
        await portfolio_manager.stop()
    except Exception as e:
        logger.error(f"Unexpected error: {e}")
        await portfolio_manager.stop()
        raise


def main():
    """Main entry point"""
    config = LivePortfolioConfig()
    try:
        logger.info("Starting Portfolio Manager Runner")
        asyncio.run(run_portfolio_manager(config))
    except KeyboardInterrupt:
        logger.info("Shutting down...")
    except Exception as e:
        logger.error(f"Fatal error: {e}")
        raise


if __name__ == "__main__":
    main() 