"""Common fixtures for all transaction tests"""

import pytest
from web3 import Web3
from eth_block_processor.txn.txn_analyzer import TransactionAnalyzer
from eth_block_processor.txn.txn_data_fetcher import TransactionDataFetcher
from eth_block_processor.txn.txn_trace_analyzer import TransactionTraceAnalyzer
from eth_block_processor.txn.txn_batch_analyzer import TransactionBatchAnalyzer


@pytest.fixture(scope="session")
def w3():
    """Initialize Web3 with retry logic"""
    provider = Web3.HTTPProvider("http://127.0.0.1:8545")
    web3 = Web3(provider)
    assert web3.is_connected(), "Web3 failed to connect to the node"
    return web3

@pytest.fixture
def txn_data_fetcher(w3):
    return TransactionDataFetcher(w3)

@pytest.fixture
def txn_analyzer(w3):
    return TransactionAnalyzer(w3)

@pytest.fixture
def txn_batch_analyzer(w3, txn_analyzer):
    return TransactionBatchAnalyzer(w3=w3) 
