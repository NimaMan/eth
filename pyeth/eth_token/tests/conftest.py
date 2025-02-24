"""Common fixtures for all token tests"""

import pytest
from web3 import Web3
from eth_block_processor.txn.txn_processor import TransactionProcessor
from eth_block_processor.txn.txn_data_fetcher import TransactionDataFetcher
from eth_token.token_manager.live_block_token_processor import LiveBlockTokenProcessor


@pytest.fixture(scope="session")
def w3():
    """Initialize Web3 with retry logic"""
    provider = Web3.HTTPProvider("http://127.0.0.1:8545")
    web3 = Web3(provider)
    assert web3.is_connected(), "Web3 failed to connect to the node"
    return web3


@pytest.fixture
def txn_data_fetcher(w3):
    """Shared transaction data fetcher"""
    return TransactionDataFetcher(w3)


@pytest.fixture
def txn_analyzer(w3):
    """Shared transaction analyzer"""
    return TransactionProcessor(w3)


@pytest.fixture
def block_live_token_processor():
    return LiveBlockTokenProcessor()
