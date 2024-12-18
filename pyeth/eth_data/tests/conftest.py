"""Common fixtures for all transaction tests"""

import pytest
from web3 import Web3
from eth_block_processor.txn.txn_analyzer import TransactionAnalyzer
from eth_block_processor.txn.txn_data_fetcher import TransactionDataFetcher

@pytest.fixture
def w3():
    return Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))

@pytest.fixture
def txn_data_fetcher(w3):
    return TransactionDataFetcher(w3)

@pytest.fixture
def txn_analyzer(w3):
    return TransactionAnalyzer(w3) 