import pytest
from eth_block_processor.alert.trading_enabled_alert import TradingEnabledAlert

@pytest.fixture
def trading_enabled_alert():
    """Fixture that returns a TradingEnabledAlert instance"""
    return TradingEnabledAlert()

@pytest.mark.asyncio
async def test_trading_enabled_alert(txn_analyzer, txn_data_fetcher, trading_enabled_alert):
    """Test that TradingEnabledAlert properly detects trading enabled events"""
    
    # Get and process the actual transaction
    txn_hash = "0x9fc6130629c69e689d6023ffb2cfbfcd7df18210e2ba97d527c9b93819c6ec5b"
    txn_data = txn_data_fetcher.get_transaction_data(txn_hash)
    
    processed_txn = txn_analyzer.process_transaction(
        txn_data['transaction'],
        txn_data['receipt'],
        txn_data['trace']
    )
    
    # Process the transaction through the alert system
    alert = await trading_enabled_alert.process_txn(processed_txn)
    
    # Verify that an alert was generated
    assert alert is not None
    assert len(alert) == 1
    
    alert = alert[0]
    assert alert.alert_type == 'Trading Enabled'
    assert alert.transaction_hash == txn_hash
    assert alert.block_number == 21423691
    assert alert.contract_address == "0x90f29ccD18c9181A9243EfF8f7546eef4b64994c"
    assert alert.from_address == "0x9e78124aDDDE586983BDD32303616A1Fb9B4F175"
    
    # Verify event details
    assert len(processed_txn.trading_enabled_events) == 1
    event = processed_txn.trading_enabled_events[0]
    assert event.token_address == "0x90f29ccD18c9181A9243EfF8f7546eef4b64994c"
    assert event.block_number == 21423691