"""Common fixtures for all transaction tests"""

import pytest
import asyncio
from aiohttp import ClientSession, TCPConnector
from web3 import Web3
from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher
from eth_data.tx_processor.tx_batch_processor import TransactionBatchProcessor
from eth_data.blockchain.block_processor import BlockProcessor


@pytest.fixture(scope="session")
def w3():
    """Initialize Web3 with retry logic"""
    provider = Web3.HTTPProvider("http://127.0.0.1:8545")
    web3 = Web3(provider)
    assert web3.is_connected(), "Web3 failed to connect to the node"
    return web3

@pytest.fixture
def tx_data_fetcher(w3):
    return TransactionDataFetcher(w3)

@pytest.fixture
def tx_analyzer(w3):
    return TransactionProcessor(w3)

@pytest.fixture
def tx_batch_analyzer(w3, tx_analyzer):
    return TransactionBatchProcessor(w3=w3)

@pytest.fixture(scope="function")
async def session():
    """Fixture to provide a shared aiohttp session"""
    connector = TCPConnector(limit=100, force_close=True)
    async with ClientSession(connector=connector) as session:
        yield session

@pytest.fixture(scope="function")
async def block_processor(session):
    """Fixture to provide a BlockProcessor with managed session"""
    processor = BlockProcessor(
        node_url="http://127.0.0.1:8545",
        session=session
    )
    yield processor
