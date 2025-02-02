import pytest
from eth_block_processor.alert.contract_creation_alert import ContractCreationAlert

@pytest.fixture
def contract_creation_alert():
    """Fixture that returns a ContractCreationAlert instance"""
    return ContractCreationAlert()

@pytest.mark.asyncio
async def test_contract_creation_alert(txn_analyzer, txn_data_fetcher, contract_creation_alert):
    """Test that ContractCreationAlert properly detects contract creation events"""
    
    # Get and process the actual transaction (SIMAI token creation)
    txn_hash = "0x45fbb2326ee70cbaacb56c12b6a14b2ab5efd41635e9d3ba9ff4fed4eee52b89"
    txn_data = txn_data_fetcher.get_transaction_data(txn_hash)
    
    processed_txn = txn_analyzer.process_transaction(
        txn_data['transaction'],
        txn_data['receipt'],
        txn_data['trace']
    )
    
    # Process the transaction through the alert system
    alerts = await contract_creation_alert.process_txn(processed_txn)
    
    # Verify that an alert was generated
    assert alerts is not None
    assert len(alerts) == 1
    
    alert = alerts[0]
    assert alert.alert_type == 'Contract Creation'
    assert alert.transaction_hash == txn_hash
    assert alert.block_number == 21423372
    assert alert.contract_address == "0x90f29ccD18c9181A9243EfF8f7546eef4b64994c"
    assert alert.creator_address == "0x9e78124aDDDE586983BDD32303616A1Fb9B4F175"
    assert alert.contract_type == "ERC20"  # This should be an ERC20 token 