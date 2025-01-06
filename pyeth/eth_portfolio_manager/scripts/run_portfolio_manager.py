"""
Portfolio Manager Runner

Objective:
---------
1. Initialize and run portfolio manager
2. Handle graceful shutdown
3. Provide CLI options for configuration
"""

import asyncio
import argparse
from datetime import datetime

from eth_portfolio_manager.core.portfolio_manager import PortfolioManager
from eth_portfolio_manager.utils.logger import get_logger


logger = get_logger(name="portfolio_manager", log_folder="portfolio_manager")


async def run_portfolio_manager():
    """Run the portfolio manager"""
    try:
        portfolio_manager = PortfolioManager(logger=logger, warmup_blocks=1000)
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
    try:
        logger.info("Starting Portfolio Manager Runner")
        asyncio.run(run_portfolio_manager())
    except KeyboardInterrupt:
        logger.info("Shutting down...")
    except Exception as e:
        logger.error(f"Fatal error: {e}")
        raise


if __name__ == "__main__":
    main() 