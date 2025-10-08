''' 
PnL Calculator for User Trade Performance using Block Processing and Token Metrics
==================================================================================

Objective:
----------
This module implements a Profit and Loss (PnL) calculator that integrates trade data from live tokens and block processing to compute the PnL for each user. The main goal is to process trade data using the LiveERC20Token instances and BlockTokenProcessor, update user performance metrics, and write calculated PnL data to the database.

Algorithm:
----------
1. Convert hardcoded dates to UNIX timestamps
2. Estimate block numbers corresponding to the timestamps using average block time
3. Initialize Web3 connection and token cache with PnL writing capability
4. Process all blocks in the specified range using HistoricalBlockTokenProcessor
5. Write PnL data to database for all tokens in the cache
6. Persist token metadata (creation_tx, trading_enabled_tx) to support token launch analytics
7. Log progress and results throughout the process

Requirement Mapping:
--------------------
- The DB schema in the data directory is used to store trade data.
- LiveERC20Token (live_token.py) and BlockTokenProcessor (block_token_processor.py) are key to updating token trade information.
- UserTokenActivityTracker (within live_token_network.py) provides user performance metrics as defined in user_activity_data_definitions.md.
- LiveTokensCache ensures only active tokens are processed, enhancing performance and relevance of the PnL calculations.
- TokenStatusWriter ensures token metadata is persisted for launch analytics and reporting.

'''

import asyncio
import traceback
import pandas as pd
from web3 import Web3

from eth_token.token_manager.block_token_processor import HistoricalBlockTokenProcessor
from eth_token.token_manager.live_tokens_cache import LiveTokensCache
from eth_token.utils.logger import get_logger
from eth_data.database.writers.token_status_writer import TokenStatusWriter


async def calculate_pnl(start_date_str, end_date_str, add_pnl_to_db=True, token_cache_size=2000, index_address_txs=False, persist_token_metadata=True):
    """
    Calculate PnL for all addresses and tokens within a date range.
    
    Args:
        start_date_str: Start date in 'YYYY-MM-DD' format
        end_date_str: End date in 'YYYY-MM-DD' format
        add_pnl_to_db: Flag to enable/disable PnL writing to database
        token_cache_size: Maximum number of tokens to hold in cache
        index_address_txs: Flag to persist address→tx participation data
        persist_token_metadata: Flag to persist token metadata (creation_tx, trading_enabled_tx)
    
    Returns:
        int: Latest processed block number or None if an error occurred
    """
    logger = get_logger(name="pnl_calculator", log_folder="pnl")
    logger.info(f"Starting PnL calculation for period {start_date_str} to {end_date_str}")
    logger.info(f"Database writing is {'enabled' if add_pnl_to_db else 'disabled'}")
    
    try:
        # Initialize Web3 connection
        logger.info("Initializing Web3 connection to local node")
        w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        
        if not w3.is_connected():
            logger.error("Failed to connect to Ethereum node")
            return None
            
        logger.info(f"Connected to Ethereum node. Current block: {w3.eth.block_number}")
        start_block, end_block = get_block_range_by_timestamp(start_date_str, end_date_str, w3, logger)
        
        if start_block is None or end_block is None:
            logger.error("Failed to estimate block range for PnL calculation")
            return None
            
        logger.info(f"Processing blocks from {start_block} to {end_block} ({end_block - start_block + 1} blocks)")
        # Initialize token cache with PnL writing capability
        logger.info(f"Initializing token cache with max size {token_cache_size}")
        token_cache = LiveTokensCache(
            max_size=token_cache_size,
            logger=logger,
            add_pnl_to_db=add_pnl_to_db
        )
        
        # Initialize block token processor
        logger.info("Setting up block token processor")
        processor = HistoricalBlockTokenProcessor(
            w3=w3,
            logger=logger,
            index_address_txs=index_address_txs
        )
        
        # Use our token cache with the processor
        processor.block_token_processor.live_tokens_cache = token_cache
        
        # Process block range
        logger.info("Beginning block processing")
        await processor.process_block_range(start_block, end_block)
        
        # Write remaining token PnL data to database if enabled
        if add_pnl_to_db:
            logger.info("Writing PnL data for remaining tokens in cache")
            tokens_processed = token_cache.write_all_token_pnl()
            logger.info(f"✓ Successfully wrote PnL data for {tokens_processed} tokens")
        else:
            logger.info("PnL writing to database is disabled")
            
        # Persist token metadata (creation_tx) and pool trading data if enabled
        if persist_token_metadata:
            logger.info("Persisting token and pool metadata to database")
            token_status_writer = TokenStatusWriter(logger=logger)
            metadata_count = 0
            pool_count = 0
            
            # Iterate through all tokens in cache and persist their metadata
            for token_address in list(token_cache.cache.keys()):
                try:
                    token = token_cache[token_address]
                    
                    # Prepare token data (without trading_enabled_tx)
                    token_data = {
                        "contract_address": token.contract_address,
                        "creator_address": token.token_data.creator_address,
                        "is_scam": token.token_data.is_scam,
                        "scam_label": token.token_data.scam_label,
                        "creation_tx": token.token_data.creation_tx
                    }
                    
                    # Prepare pools data with trading_enabled info
                    pools_data = {}
                    if hasattr(token, 'pool_manager') and token.pool_manager:
                        for pool_address, pool in token.pool_manager.pools.items():
                            pools_data[pool_address] = {
                                "pool_type": pool.pool_type,
                                "denom_address": pool.denom_address,
                                "fee_tier": getattr(pool, 'fee_tier', None),
                                "is_scam": token.token_data.is_scam,
                                "scam_label": token.token_data.scam_label,
                                "trading_enabled": pool.trading_enabled,
                                "trading_enabled_block": pool.trading_enabled_block,
                                "trading_enabled_tx": pool.trading_enabled_tx
                            }
                            pool_count += 1
                    
                    # Use create_or_update_token_with_pools to persist both token and pool data
                    if pools_data:
                        if token_status_writer.create_or_update_token_with_pools(token_data, pools_data):
                            metadata_count += 1
                    else:
                        # No pools, just update token
                        if token_status_writer.create_or_update_token(token_data):
                            metadata_count += 1
                            
                except Exception as e:
                    logger.error(f"Error persisting metadata for token {token_address}: {e}")
                    
            logger.info(f"✓ Successfully persisted metadata for {metadata_count} tokens and {pool_count} pools")
        
        latest_block = processor.block_token_processor.latest_processed_block
        logger.info(f"PnL calculation complete for blocks {start_block} to {latest_block}")
        return latest_block
        
    except Exception as e:
        logger.error(f"Error during PnL calculation: {str(e)}")
        logger.error(traceback.format_exc())
        return None


def get_block_range_by_timestamp(start_date_str, end_date_str, w3, logger=None):
    """
    Estimate block number for a given timestamp using average block time.
    
    Args:
        timestamp: UNIX timestamp
        logger: Logger instance (optional)
    
    Returns:
        int: Estimated block number
    """
    try:
        start_timestamp = pd.Timestamp(start_date_str).timestamp()
        end_timestamp = pd.Timestamp(end_date_str).timestamp()
        BLOCKS_PER_DAY = 7200  # Approximate number of blocks per day (based on ~12s block time)
        SECONDS_PER_DAY = 86400  # Seconds in a day
        latest_block = w3.eth.block_number
        current_timestamp = pd.Timestamp.now().timestamp()
        # Calculate days difference
        start_days_diff = (current_timestamp - start_timestamp) / SECONDS_PER_DAY
        end_days_diff = (current_timestamp - end_timestamp) / SECONDS_PER_DAY
        estimated_start_block = latest_block - int(start_days_diff * BLOCKS_PER_DAY)
        estimated_end_block = latest_block - int(end_days_diff * BLOCKS_PER_DAY)
        
        if logger:
            logger.info(f"Estimated start block {estimated_start_block} for timestamp {start_timestamp}")
            logger.info(f"Estimated end block {estimated_end_block} for timestamp {end_timestamp}")
            logger.info(f"Days difference from now: {start_days_diff:.2f} days")
            
        return estimated_start_block, estimated_end_block
    
    except Exception as e:
        if logger:
            logger.error(f"Error estimating block for timestamp {start_timestamp} to {end_timestamp}: {str(e)}")
        return None


if __name__ == "__main__":
    start_date = "2024-10-01"
    end_date = "2025-08-18"
    add_pnl_to_db = True
    index_address_txs = False
    persist_token_metadata = True  # Enable token metadata persistence
    cache_size = 2000
    # Run the PnL calculation
    result = asyncio.run(calculate_pnl(
        start_date, 
        end_date, 
        add_pnl_to_db=add_pnl_to_db,
        token_cache_size=cache_size,
        index_address_txs=index_address_txs,
        persist_token_metadata=persist_token_metadata
    ))
    
    if result is None:
        print("PnL calculation failed. Check logs for details.")
    else:
        print(f"PnL calculation completed successfully. Latest processed block: {result}")
