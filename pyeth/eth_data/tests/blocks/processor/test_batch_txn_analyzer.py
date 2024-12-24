import pytest
import asyncio

# Real transaction hashes that had issues
TEST_TRANSACTIONS = [
    "0x1ae605ff25d25124a6b985285a1ba42758cbbe9734eb1360eedd5e482a27650a",
    "0x80ab120192fa566ad22ba5828b1aa69ffa447a00ba9e56e651ab25d7779ddeff",
    "0x90a61290ad3d1d5d16e0f4dea33ac938d84f254c9529831d83e98a8bb0f936e3"
]

@pytest.fixture
def real_transaction_data(w3):
    """Fetch real transaction data for testing"""
    tx_data = {}
    for tx_hash in TEST_TRANSACTIONS:
        tx = w3.eth.get_transaction(tx_hash)
        receipt = w3.eth.get_transaction_receipt(tx_hash)
        tx_data[tx_hash] = {
            'transaction': dict(tx),
            'receipt': dict(receipt)
        }
    return tx_data

@pytest.mark.asyncio
async def test_batch_analyzer_with_real_transactions(w3, txn_batch_analyzer, real_transaction_data):
    """Test batch analyzer with real transaction data that previously failed"""
    
    for tx_hash, tx_data in real_transaction_data.items():
        for data_type in ['raw', 'dict']:
            if data_type == 'dict':
                tx_data = {
                    'transaction': dict(tx_data['transaction']),
                    'receipt': dict(tx_data['receipt'])
                }
                
            result = await txn_batch_analyzer._analyze_single_transaction_safe(
                transaction=tx_data['transaction'],
                receipt=tx_data['receipt']
            )
            
            # Basic sanity checks
            assert result.hash == tx_hash
            