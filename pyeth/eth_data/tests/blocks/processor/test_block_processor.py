import pytest
import pytest_asyncio
from web3 import AsyncWeb3, AsyncHTTPProvider
from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_block_processor.utils.logger import get_logger

logger = get_logger(name="test_block_processor", log_folder="tests")

@pytest_asyncio.fixture
async def web3_instance():
    """
    Create an AsyncWeb3 instance for testing.
    
    Yields:
        AsyncWeb3: An instance connected to the specified Ethereum node.
    """
    node_url = "http://127.0.0.1:8545"
    w3 = AsyncWeb3(AsyncHTTPProvider(node_url))
    try:
        # Verify connection by fetching the latest block number
        latest_block = await w3.eth.block_number
        logger.info(f"Connected to Ethereum node. Latest block number: {latest_block}")
        yield w3
    finally:
        # AsyncHTTPProvider does not support disconnecting, so we skip this step
        logger.info("Teardown for web3_instance fixture completed.")

@pytest_asyncio.fixture
async def block_processor(web3_instance):
    """
    Create a BlockProcessor instance for testing.
    
    Args:
        web3_instance (AsyncWeb3): The Web3 instance to be used by the BlockProcessor.
    
    Yields:
        BlockProcessor: An instance of BlockProcessor initialized with the node URL.
    """
    node_url = "http://127.0.0.1:8545"
    processor = BlockProcessor(node_url=node_url)
    try:
        yield processor
    finally:
        # If BlockProcessor has an async close method, call it here
        if hasattr(processor, 'close') and callable(getattr(processor, 'close')):
            await processor.close()
            logger.info("BlockProcessor instance closed.")
        else:
            logger.info("No cleanup required for BlockProcessor.")

@pytest.mark.asyncio
async def test_process_single_block(block_processor, web3_instance):
    """
    Test processing a single Ethereum block to ensure the BlockProcessor works without errors.
    
    Args:
        block_processor (BlockProcessor): The BlockProcessor instance for processing blocks.
        web3_instance (AsyncWeb3): The Web3 instance for interacting with the Ethereum node.
    
    Assertions:
        - processed_block is a list.
        - processed_block contains at least one transaction.
    """
    try:
        # Get the latest block number
        latest_block = await web3_instance.eth.block_number
        test_block_number = latest_block - 5  # Use a recent but confirmed block

        logger.info(f"Testing block processing for block {test_block_number}")

        # Fetch block data
        block_data = await web3_instance.eth.get_block(test_block_number, full_transactions=True)

        # Process the block
        processed_block = await block_processor.process_block(
            block_number=test_block_number,
            block_data=block_data
        )

        # Verify the processed block
        assert isinstance(processed_block, list), "Processed block should be a list of transactions"
        assert len(processed_block) > 0, "Processed block should contain at least one transaction"

        # Log the results
        logger.info(f"Successfully processed block {test_block_number}")
        logger.info(f"Number of transactions: {len(processed_block)}")

    except Exception as e:
        logger.error(f"Error in test_process_single_block: {e}", exc_info=True)
        raise