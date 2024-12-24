import pytest
from eth_block_processor.alert.txn_alert_processor import TransactionAlertProcessor

def test_txn_alert_processor(w3, txn_data_fetcher, txn_analyzer):
    txn_hash = "0xe1dbcab7c44722957ce64db87afffd8bf0ac389105cb088a4cd44fb570579c4a"
    txn_data = txn_data_fetcher.get_transaction_data(txn_hash)
    
    txn_analyzer.analyze_transaction(txn_data['transaction'], txn_data['receipt'], txn_data['trace'])
    
    TransactionAlertProcessor().process_transaction(txn_data)


if __name__ == "__main__":  
    pytest.main()