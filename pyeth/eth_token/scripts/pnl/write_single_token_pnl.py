"""
Write Single Token PnL to Database
----------------------------------
Objective: Analyze a single token contract, calculate its PnL metrics,
           and write the results to the database.

Algorithm:
1.  Accept a token contract address via command-line argument.
2.  Initialize Web3, TokenAnalyzer, and TokenPnLWriter.
3.  Create a LiveTokensCache instance configured for DB writing.
4.  Build the LiveERC20Token using TokenAnalyzer.
5.  Add the token to the cache.
6.  Call cache.write_all_token_pnl() to trigger the database write
    via the associated TokenPnLWriter.
7.  Log progress and results.
8.  Check if trades have latest_block field populated.
"""

import asyncio
from datetime import datetime
from web3 import Web3
from sqlalchemy import text
from eth_data.database.eth_db_conn import get_db_session_maker

from eth_token.token_builder.live_token_builder import LiveTokenBuilder
from eth_data.database.writers.token_status_writer import TokenStatusWriter
from eth_token.utils.logger import get_logger # Use a consistent logger


def check_trades_latest_block(token_address: str):
    """
    Check if trades for a given token have the latest_block field populated.
    
    Args:
        token_address: The contract address of the ERC20 token.
        
    Returns:
        tuple: (total_trades, trades_without_latest_block)
    """
    logger = get_logger("check_trades_latest_block")
    
    with get_db_session_maker(db='eth_db')() as session:
        # Get total number of trades for this token
        total_query = text("""
            SELECT COUNT(*) 
            FROM eth_db.trades 
            WHERE token_address = :token_address
        """)
        total_trades = session.execute(total_query, {"token_address": token_address}).scalar_one()
        
        # Get number of trades without latest_block
        missing_query = text("""
            SELECT COUNT(*) 
            FROM eth_db.trades 
            WHERE token_address = :token_address 
            AND latest_block IS NULL
        """)
        trades_without_latest_block = session.execute(missing_query, {"token_address": token_address}).scalar_one()
        
        logger.info(f"Token {token_address} has {total_trades} total trades")
        logger.info(f"Of these, {trades_without_latest_block} trades are missing latest_block")
        
        return total_trades, trades_without_latest_block


async def write_single_token_pnl_to_db(token_address: str):
    """
    Analyzes a single token using LiveTokenBuilder and writes token + pool data to DB.

    Args:
        token_address: The contract address of the ERC20 token.
    """
    logger = get_logger("single_token_pnl_writer", log_folder="pnl")
    logger.info(f"Starting token analysis and DB write for token: {token_address}")
    logger.info(f"Script started at: {datetime.now()}")

    # --- Initialization ---
    logger.info("Initializing components...")
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    if not w3.is_connected():
        logger.error("Failed to connect to Ethereum node.")
        return

    logger.info(f"Connected to Ethereum node. Current block: {w3.eth.block_number}")

    # Initialize LiveTokenBuilder
    builder = LiveTokenBuilder(logger=logger, w3=w3)
    logger.info("LiveTokenBuilder initialized.")

    # --- Token Building ---
    logger.info(f"Building token {token_address} with complete transaction history...")
    live_token = await builder.build_token(
        contract_address=token_address,
        max_blocks=2000  # Limit for faster testing
    )

    if not live_token:
        logger.error(f"Failed to build token {token_address}.")
        return

    logger.info(f"Token built successfully:")
    logger.info(f"  Name: {live_token.name}")
    logger.info(f"  Symbol: {live_token.symbol}")
    logger.info(f"  Decimals: {live_token.decimals}")
    logger.info(f"  Total supply: {live_token.total_supply}")
    logger.info(f"  Number of pools: {len(live_token.pool_manager.pools) if live_token.pool_manager else 0}")

    # --- Prepare Data for Database ---
    logger.info("Preparing token and pool data for database...")
    
    # Prepare token data (without trading_enabled_tx)
    token_data = {
        "contract_address": live_token.contract_address,
        "creator_address": live_token.token_data.creator_address,
        "is_scam": live_token.token_data.is_scam,
        "scam_label": live_token.token_data.scam_label,
        "creation_tx": live_token.token_data.creation_tx
    }

    # Prepare pools data with trading_enabled info
    pools_data = {}
    if hasattr(live_token, 'pool_manager') and live_token.pool_manager:
        logger.info("Extracting pool data:")
        for pool_address, pool in live_token.pool_manager.pools.items():
            pool_type = type(pool).__name__.replace('Pool', '').replace('Uniswap', '')  # Extract pool type
            logger.info(f"  Pool {pool_address}:")
            logger.info(f"    Type: {pool_type}")
            logger.info(f"    Trading enabled: {pool.trading_enabled}")
            logger.info(f"    Trading enabled block: {pool.trading_enabled_block}")
            logger.info(f"    Trading enabled tx: {pool.trading_enabled_tx}")
            
            pools_data[pool_address] = {
                "pool_type": pool_type,
                "denom_address": pool.denom_address,
                "fee_tier": getattr(pool, 'fee_tier', None),
                "is_scam": live_token.token_data.is_scam,
                "scam_label": live_token.token_data.scam_label,
                "trading_enabled": pool.trading_enabled,
                "trading_enabled_block": pool.trading_enabled_block,
                "trading_enabled_tx": pool.trading_enabled_tx
            }

    # --- Database Write ---
    logger.info("Writing token and pool data to database...")
    token_status_writer = TokenStatusWriter(logger=logger)

    if pools_data:
        success = token_status_writer.create_or_update_token_with_pools(token_data, pools_data)
        if success:
            logger.info(f"✓ Successfully persisted token and {len(pools_data)} pools to database")
        else:
            logger.error("✗ Failed to persist token and pool data")
    else:
        success = token_status_writer.create_or_update_token(token_data)
        if success:
            logger.info("✓ Successfully persisted token (no pools found)")
        else:
            logger.error("✗ Failed to persist token data")

    # --- Check latest_block field ---
    logger.info("Checking latest_block field for trades...")
    total_trades, trades_without_latest_block = check_trades_latest_block(token_address)
    
    if trades_without_latest_block > 0:
        logger.warning(f"Found {trades_without_latest_block} trades without latest_block field")
        logger.warning("These trades need to be updated with the latest block information")
    else:
        logger.info("All trades have latest_block field populated")

    logger.info(f"Script finished at: {datetime.now()}")


if __name__ == "__main__":
    # Use the original test token
    contract_address = "0xd6600a22080e6c508C27e0A687b68f7CBb89DB11"
    # Run the main async function
    asyncio.run(write_single_token_pnl_to_db(contract_address))