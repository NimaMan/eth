"""Common fixtures for all transaction tests"""

import pytest
from aiohttp import ClientSession, TCPConnector
from web3 import Web3


@pytest.fixture(scope="session")
def w3():
    """Initialize Web3 with retry logic"""
    provider = Web3.HTTPProvider("http://127.0.0.1:8545")
    web3 = Web3(provider)
    assert web3.is_connected(), "Web3 failed to connect to the node"
    return web3

@pytest.fixture(scope="function")
async def session():
    """Fixture to provide a shared aiohttp session"""
    connector = TCPConnector(limit=100, force_close=True)
    async with ClientSession(connector=connector) as session:
        yield session
