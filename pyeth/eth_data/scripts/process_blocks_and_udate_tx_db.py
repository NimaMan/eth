import asyncio
from tqdm import tqdm
from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_block_processor.utils.logger import get_logger
from datetime import datetime, timezone
from sqlalchemy import text
from web3 import Web3
from sarigoz.utils.time_block_converter import TimeBlockConverter
from sarigoz.data.db.eth_db_conn import get_db_engine


async def process_blocks_in_range(
    node_url: str,
    num_blocks: int = 1000000,
    save_txn_to_db: bool = True,
    start_block: int = None,
    end_block: int = None
):
    """
    Process Ethereum blocks sequentially, skipping already processed blocks.
    
    Args:
        node_url: URL of the Ethereum node
        num_blocks: Number of blocks to process
        save_txn_to_db: Whether to save transactions to the database
        start_block: Starting block number (optional)
        end_block: Ending block number (optional)
    """
    logger = get_logger(name="db_tx_processor")
    
    # Initialize processor with database saving enabled
    processor = BlockProcessor(
        node_url=node_url,
        save_txn_to_db=save_txn_to_db,
        logger=logger
    )
    
    try:
        # Determine block range
        if end_block is None:
            # Get latest block number
            latest_block = await processor.block_fetcher.w3.eth.get_block_number()
            end_block = latest_block
        
        if start_block is None:
            # Calculate start block based on the number of blocks to process
            start_block = max(0, end_block - num_blocks + 1)
        
        logger.info(f"Block range: {start_block} to {end_block}")        
        for block_number in tqdm(range(end_block, start_block - 1, -1)):    
            result = await processor.process_block(block_number)
        
    except Exception as e:
        logger.error(f"Error in process_blocks: {str(e)}")
        raise


def find_missing_blocks(start_date_str: str, node_url: str = "http://localhost:8545") -> list[int]:
    """
    Identifies missing block numbers in the eth_db.blocks table for a given date range.

    Args:
        start_date_str: The start date in 'YYYY-MM-DD' format.
        node_url: URL of the Ethereum node to connect to.

    Returns:
        A list of missing block numbers.
    """
    logger = get_logger("find_missing_blocks")
    missing_blocks = []
    engine = None
    w3 = None

    try:
        # --- Date and Block Range Calculation ---
        start_dt = datetime.strptime(start_date_str, "%Y-%m-%d").replace(tzinfo=timezone.utc)
        start_ts = int(start_dt.timestamp())

        # Connect to Web3 to get the latest block and converter
        logger.info(f"Connecting to Ethereum node at {node_url}...")
        w3 = Web3(Web3.HTTPProvider(node_url))
        if not w3.is_connected():
            logger.error("Failed to connect to Ethereum node.")
            return []

        latest_block_num = w3.eth.get_block_number()
        logger.info(f"Latest block number from node: {latest_block_num}")

        block_time_converter = TimeBlockConverter(w3=w3)
        start_block = block_time_converter.timestamp_to_block(start_ts)
        # Use the latest block from the node as the end block
        end_block = latest_block_num

        if start_block > end_block:
            logger.warning(f"Start block {start_block} is after the latest block {end_block}. No range to check.")
            return []

        logger.info(f"Checking for missing blocks from {start_date_str} to now (block range: {start_block} to {end_block})")

        # --- Database Query ---
        logger.info("Connecting to the database...")
        engine = get_db_engine(db='eth_db') # Ensure this matches your database name if different

        with engine.connect() as connection:
            logger.info(f"Querying existing blocks between {start_block} and {end_block}...")
            query = text("""
                SELECT block_number
                FROM eth_db.blocks
                WHERE block_number >= :start_block AND block_number <= :end_block
                ORDER BY block_number
            """)
            result = connection.execute(query, {"start_block": start_block, "end_block": end_block})
            existing_blocks_set = {row[0] for row in result} # Use column index 0
            logger.info(f"Found {len(existing_blocks_set)} blocks in the database within the range.")

        # --- Comparison ---
        logger.info("Comparing expected blocks with existing blocks...")
        full_block_range_set = set(range(start_block, end_block + 1))

        missing_blocks_set = full_block_range_set - existing_blocks_set
        missing_blocks = sorted(list(missing_blocks_set))

        logger.info(f"Identified {len(missing_blocks)} missing blocks.")

    except Exception as e:
        logger.error(f"An error occurred: {e}", exc_info=True)
        # Optionally re-raise or handle differently

    finally:
        # Clean up resources if needed (SQLAlchemy engine handles pooling)
        pass

    return missing_blocks


async def process_blocks_batch(block_numbers: list[int]):
    logger = get_logger("db_tx_processor")
    block_processor = BlockProcessor(
        node_url="http://localhost:8545",
        save_txn_to_db=True,
        logger=logger
    )
    for block_number in tqdm(block_numbers):
        await block_processor.process_block(block_number)

def update_tx_db_with_missing_blocks(start_date_str="2025-01-01"):
    missing_blocks = find_missing_blocks(start_date_str)
    if missing_blocks:
        asyncio.run(process_blocks_batch(missing_blocks))



def run_process_blocks_in_range(
    node_url="http://localhost:8545",
    num_blocks=1000000,
    save_txn_to_db=True,
    start_block=None,
    end_block=None
):
    asyncio.run(process_blocks_in_range(
        node_url=node_url,
        num_blocks=num_blocks,
        save_txn_to_db=save_txn_to_db,
        start_block=start_block,
        end_block=end_block
    ))


if __name__ == "__main__":
    update_tx_db_with_missing_blocks(start_date_str="2025-01-01")