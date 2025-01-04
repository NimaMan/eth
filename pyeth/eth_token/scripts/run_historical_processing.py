"""
Test Historical Processing and Token Access

Objective:
---------
1. Process last 10000 blocks historically
2. Store token states in cache
3. Test token access via LiveTokenProvider
4. Verify token data consistency
"""

import asyncio
import sys
from typing import Optional
from eth_token_monitor.token_manager.block_token_processor import BlockTokenProcessor
from eth_token_monitor.services.api.live_token_provider import LiveTokenProvider
from eth_token_monitor.utils.logger import get_logger
from eth_block_processor.blockchain.block_processor import BlockProcessor


async def process_historical_blocks(
    end_block: Optional[int] = None,
    block_range: int = 10000,
    redis_url: str = "redis://localhost:6379/0",
    logger=None
):
    """Process historical blocks and store token states"""
    
    logger = logger or get_logger(name="historical_test", log_folder="tokens_live")
    block_processor = BlockProcessor(logger=logger)
    
    # Get latest block if not specified
    if not end_block:
        end_block = await block_processor.block_fetcher.fetch_latest_block_number()
    
    start_block = end_block - block_range
    logger.info(f"Processing blocks {start_block} to {end_block}")
    
    # Initialize token processor
    token_processor = BlockTokenProcessor(redis_url=redis_url, logger=logger)
    
    try:
        # Process blocks in batches
        await token_processor.process_block_range(
            start_block=start_block,
            end_block=end_block,
        )
        
        return token_processor.latest_processed_block
        
    except Exception as e:
        logger.error(f"Error processing historical blocks: {e}")
        return None


async def test_token_access(latest_block: int, logger=None):
    """Test accessing processed tokens"""
    
    logger = logger or get_logger(name="historical_test", log_folder="tokens_live")
    token_provider = LiveTokenProvider()
    
    try:
        # Get active tokens
        active_tokens = await token_provider.get_active_tokens(limit=10)
        logger.info(f"Found {len(active_tokens)} active tokens")
        
        # Test accessing each token
        for token in active_tokens:
            logger.info(f"\nToken: {token.contract_address}")
            logger.info(f"Age: {token.token_age_blocks} blocks")
            logger.info(f"Is scam: {token.is_scam}")
            if token.is_scam:
                logger.info(f"Scam label: {token.scam_label}")
            
            # Test getting same token directly
            retrieved_token = await token_provider.get_token(token.contract_address)
            if retrieved_token:
                logger.info("Successfully retrieved token directly")
                
            # Verify token data consistency
            assert retrieved_token.token_age_blocks == token.token_age_blocks, "Token age mismatch"
            assert retrieved_token.is_scam == token.is_scam, "Scam status mismatch"
            
    except Exception as e:
        logger.error(f"Error testing token access: {e}")


async def main():
    logger = get_logger(name="historical_test", log_folder="tokens_live")
    
    try:
        # Process historical blocks
        latest_block = await process_historical_blocks(
            block_range=10000,
            batch_size=100,
            logger=logger
        )
        
        if latest_block:
            logger.info(f"Successfully processed blocks up to {latest_block}")
            
            # Test token access
            await test_token_access(latest_block, logger)
        else:
            logger.error("Failed to process historical blocks")
            
    except Exception as e:
        logger.error(f"Error in main process: {e}")
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main()) 