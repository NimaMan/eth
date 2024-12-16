import pytest
from web3 import Web3
from eth_block_processor.txn.txn_batch_analyzer import TransactionBatchAnalyzer
from eth_block_processor.txn.txn_analyzer import TransactionAnalyzer

# Real transaction hashes that had issues
TEST_TRANSACTIONS = [
    "0x1ae605ff25d25124a6b985285a1ba42758cbbe9734eb1360eedd5e482a27650a",
    "0x80ab120192fa566ad22ba5828b1aa69ffa447a00ba9e56e651ab25d7779ddeff",
    "0x90a61290ad3d1d5d16e0f4dea33ac938d84f254c9529831d83e98a8bb0f936e3"
]

@pytest.fixture
def w3():
    return Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))

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

def test_batch_analyzer_with_real_transactions(real_transaction_data):
    """Test batch analyzer with real transaction data that previously failed"""
    transaction_analyzer = TransactionAnalyzer(None)
    batch_analyzer = TransactionBatchAnalyzer(transaction_analyzer)
    
    for tx_hash, tx_data in real_transaction_data.items():
        # Test with both AttributeDict (direct from web3) and dict (converted) data
        for data_type in ['raw', 'dict']:
            if data_type == 'dict':
                # Convert to regular dict
                tx_data = {
                    'transaction': dict(tx_data['transaction']),
                    'receipt': dict(tx_data['receipt'])
                }
                
                # Add some pre-converted values to test mixed types
                if 'blockNumber' in tx_data['transaction']:
                    tx_data['transaction']['blockNumber'] = int(tx_data['transaction']['blockNumber'], 16)
            
            result = batch_analyzer._analyze_single_transaction(
                transaction=tx_data['transaction'],
                receipt=tx_data['receipt']
            )
            
            # Basic sanity checks
            assert result.hash == tx_hash
            assert isinstance(result.block_number, int)
            assert isinstance(result.value, int)
            assert isinstance(result.fees.gas_used, int)
            assert isinstance(result.fees.gas_price, int)
