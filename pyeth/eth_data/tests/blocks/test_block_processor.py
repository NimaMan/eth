import asyncio
import pytest
import pytest_asyncio
from web3 import AsyncWeb3, AsyncHTTPProvider
from pyreth import block_processor as pyreth_block_processor

from eth_data.utils.logger import get_logger

logger = get_logger(name="test_block_processor", log_folder="tests")

@pytest_asyncio.fixture
async def web3_instance(session):
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
        logger.info("Teardown for web3_instance fixture completed.")

@pytest_asyncio.fixture
async def block_processor(web3_instance):
    """
    Create a BlockProcessor instance for testing.
    
    Args:
        web3_instance (AsyncWeb3): The Web3 instance to be used by the BlockProcessor.
        session (ClientSession): The shared aiohttp session.
    
    Yields:
        BlockProcessor: An instance of BlockProcessor initialized with the node URL.
    """
    yield pyreth_block_processor()

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

        processed_block_result = await asyncio.to_thread(
            block_processor.process_block,
            test_block_number,
        )

        # Verify the processed block
        assert hasattr(processed_block_result, "transactions"), "Processed block should expose transactions"
        assert len(processed_block_result.transactions) > 0, "Processed block should contain at least one transaction"

        # Log the results
        logger.info(f"Successfully processed block {test_block_number}")
        logger.info(f"Number of transactions: {len(processed_block_result.transactions)}")

    except Exception as e:
        logger.error(f"Error in test_process_single_block: {e}", exc_info=True)
        raise

@pytest.mark.asyncio
async def test_batch_block_processing():
    """Test complete batch processing flow"""
    # Setup
    processor = pyreth_block_processor()
    
    try:
        # Get a range of recent blocks
        latest_block = processor.get_latest_block()
        start_block = latest_block - 10  # Test with 10 blocks
        
        print(f"\nTesting batch processing for blocks {start_block} to {latest_block}")
        
        # Process blocks
        processed_blocks = {
            block_number: await asyncio.to_thread(processor.process_block, block_number)
            for block_number in range(start_block, latest_block + 1)
        }
        
        # Verify results
        assert len(processed_blocks) > 0, "No blocks processed"
        
        # Check data completeness for each block
        for block_num, block_data in processed_blocks.items():
            assert hasattr(block_data, "transactions"), f"Invalid data format for block {block_num}"
            
            # Verify each transaction was analyzed
            for tx_result in block_data.transactions:
                if isinstance(tx_result, Exception):
                    print(f"Error processing transaction: {tx_result}")
                    continue
                    
                assert hasattr(tx_result, 'hash'), "Transaction missing hash"
                assert hasattr(tx_result, 'from_address'), "Transaction missing from_address"
                assert hasattr(tx_result, 'to_address'), "Transaction missing to_address"
                assert hasattr(tx_result, 'value'), "Transaction missing value"
        
        print(f"\nSuccessfully processed {len(processed_blocks)} blocks")
        
    except Exception as e:
        print(f"Test failed: {str(e)}")
        raise


if __name__ == "__main__":
    asyncio.run(test_batch_block_processing())
