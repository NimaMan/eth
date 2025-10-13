"""
Transaction Analyzer Test Suite - Real Transaction Tests
"""

import pytest
import asyncio
import numpy as np
from eth_data.tx_processor.data_models.tx_models import ProcessedTransaction, TransactionFees

@pytest.fixture
def expected_eth_transfer():
    """Expected DetailedTransaction object for the ETH transfer"""
    return ProcessedTransaction(
        hash="0x0d6a7c23ba11f31a01cab82d8ae0b770286d7a189828e57d3f871bed4e0480f3",
        block_number=21423357,
        tx_index=37,
        from_address="0xc6c66cb4EC3b80159D36F0A566491450de1F5731",
        to_address="0x9e78124aDDDE586983BDD32303616A1Fb9B4F175",
        contract_address=None,
        value=np.float64(2.4),  # ETH value
        status=1,
        nonce=5,
        input="0x",
        tx_type="Ether Transfer",
        fees=TransactionFees(
            gas_price=42070009906,  # 42.070009906 Gwei
            gas_used=21000,
            tx_fee=np.float64(0.000883470208026)  # 21000 * 42070009906 / 1e18
        ),
        unique_addresses={
            "0xc6c66cb4EC3b80159D36F0A566491450de1F5731",
            "0x9e78124aDDDE586983BDD32303616A1Fb9B4F175"
        }
    )

def test_eth_transfer_analysis(tx_analyzer, tx_data_fetcher, expected_eth_transfer):
    """Test that both sync and async analysis match the expected transaction details"""
    
    # Get actual transaction data
    tx_hash = "0x0d6a7c23ba11f31a01cab82d8ae0b770286d7a189828e57d3f871bed4e0480f3"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)
    
    # Test synchronous analysis
    sync_result = tx_analyzer.process_transaction(
        tx_data['transaction'],
        tx_data['receipt'],
        tx_data['trace']
    )
    assert sync_result == expected_eth_transfer
    
    # Test asynchronous analysis
    async def run_async_analysis():
        return await tx_analyzer.process_transaction_async(
            tx_data['transaction'],
            tx_data['receipt'],
            tx_data['trace']
        )
    
    async_result = asyncio.run(run_async_analysis())
    assert async_result == expected_eth_transfer
