


import pytest
from eth_block_processor.alert.user_involved_alert import OrcaAlert
@pytest.fixture
def scam_orca_alert():
    """Fixture that returns a TradingEnabledAlert instance"""
    return OrcaAlert()

@pytest.mark.asyncio
async def test_scam_orca_alert(txn_analyzer, txn_data_fetcher, scam_orca_alert):
    """Test that TradingEnabledAlert properly detects trading enabled events"""
    
    # Get and process the actual transaction
    txn_hash = "0xb900e46fca6688111bbb521c2509317986b4d9591de5710ba939c2569f16df5f"
    txn_data = txn_data_fetcher.get_transaction_data(txn_hash)
    
    processed_txn = txn_analyzer.analyze_transaction(
        txn_data['transaction'],
        txn_data['receipt'],
        txn_data['trace']
    )
    
    # Process the transaction through the alert system
    alert = await scam_orca_alert.process_txn(processed_txn)
    
    # Verify that an alert was generated
    assert alert is not None
    assert len(alert) == 1
    
    alert = alert[0]
    assert alert.alert_type == 'Orca Scam'
    assert alert.transaction_hash == txn_hash
    