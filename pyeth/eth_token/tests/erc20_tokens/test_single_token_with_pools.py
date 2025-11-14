"""
Test Single Token with Pool Persistence
---------------------------------------
Build a token using LiveTokenBuilder and persist both token and pool metadata.
"""

import asyncio
from web3 import Web3
from eth_token.token_builder.live_token_builder import LiveTokenBuilder
from eth_data.database.writers.token_status_writer import TokenStatusWriter
from eth_token.utils.logger import get_logger


async def test_token_with_pool_persistence(token_address: str):
    """Test building a token and persisting it with pool data."""
    logger = get_logger("test_token_pools", log_folder="test")
    
    logger.info(f"Testing token {token_address} with pool persistence")
    
    # Initialize Web3 and builder
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    if not w3.is_connected():
        logger.error("Failed to connect to Ethereum node.")
        return
    
    logger.info(f"Connected to Ethereum node. Current block: {w3.eth.block_number}")
    
    # Build token with LiveTokenBuilder
    builder = LiveTokenBuilder(logger=logger, w3=w3)
    
    logger.info("Building token with LiveTokenBuilder...")
    token = await builder.build_token(
        contract_address=token_address,
        max_blocks=1000  # Limit to recent blocks for testing
    )
    
    if not token:
        logger.error(f"Failed to build token {token_address}")
        return
    
    logger.info(f"Token built successfully:")
    logger.info(f"  Name: {token.name}")
    logger.info(f"  Symbol: {token.symbol}")
    logger.info(f"  Decimals: {token.decimals}")
    logger.info(f"  Number of pools: {len(token.pool_manager.pools) if token.pool_manager else 0}")
    
    # Prepare token data
    token_data = {
        "contract_address": token.contract_address,
        "creator_address": token.creator_address,
        "is_scam": token.is_scam,
        "scam_label": token.scam_label,
        "creation_tx": token.creation_tx
    }
    
    # Prepare pools data with trading_enabled info
    pools_data = {}
    if hasattr(token, 'pool_manager') and token.pool_manager:
        logger.info("Extracting pool data:")
        for pool_address, pool in token.pool_manager.pools.items():
            logger.info(f"  Pool {pool_address}: trading_enabled={pool.trading_enabled}, "
                       f"trading_enabled_tx={pool.trading_enabled_tx}")
            
            pools_data[pool_address] = {
                "pool_type": pool.pool_type,
                "denom_address": pool.denom_address,
                "fee_tier": pool.fee_tier if hasattr(pool, 'fee_tier') else None,
                "is_scam": token.is_scam,
                "scam_label": token.scam_label,
                "trading_enabled": pool.trading_enabled,
                "trading_enabled_block": pool.trading_enabled_block,
                "trading_enabled_tx": pool.trading_enabled_tx
            }
    
    # Persist to database
    logger.info("Persisting token and pool data to database...")
    token_status_writer = TokenStatusWriter(logger=logger)
    
    if pools_data:
        success = token_status_writer.create_or_update_token_with_pools(token_data, pools_data)
        if success:
            logger.info(f"✓ Successfully persisted token and {len(pools_data)} pools")
        else:
            logger.error("Failed to persist token and pool data")
    else:
        success = token_status_writer.create_or_update_token(token_data)
        if success:
            logger.info("✓ Successfully persisted token (no pools found)")
        else:
            logger.error("Failed to persist token data")
    
    logger.info("Test completed")


if __name__ == "__main__":
    # Use a token that should have pools
    test_token = "0xA0b86a33E6441936be8b8e0d8c5fBD57f2B98b5A"  # WBTC - should have pools
    
    asyncio.run(test_token_with_pool_persistence(test_token))
