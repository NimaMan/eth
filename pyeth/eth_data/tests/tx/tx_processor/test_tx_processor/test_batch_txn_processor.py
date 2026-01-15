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

def test_batch_analyzer_with_real_transactions(tx_batch_analyzer, real_transaction_data):
    """Test batch analyzer with real transaction data that previously failed"""
    
    for tx_hash, payload in real_transaction_data.items():
        # Run against both the original web3 objects and a plain dict clone
        for use_copy in (False, True):
            tx_payload = payload if not use_copy else {
                'transaction': dict(payload['transaction']),
                'receipt': dict(payload['receipt'])
            }
            result = asyncio.run(
                tx_batch_analyzer._process_single_transaction(
                    transaction=tx_payload['transaction'],
                    receipt=tx_payload['receipt']
                )
            )
            assert result.hash == tx_hash
            
