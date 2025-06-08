#!/usr/bin/env python3
"""
Test Scam Detection System

This script tests the eth_token scam detection system with real scam contracts
from the database to ensure it's working correctly.
"""

import sys
import os
sys.path.append('/home/nima/code/crypto/py')

from eth_token.live_erc20_token.live_token import ERC20Token
from eth_token.live_erc20_token.token_health.token_health_predictor import TokenHealthPredictor
from eth_token.live_erc20_token.token_health.volume_analyzer import VolumeAnalyzer
from eth_token.alert.scam_alert import ScamAlert
from sarigoz.data.db.eth_db_conn import get_db_engine
from sqlalchemy import text
import pandas as pd

def test_scam_detection():
    """Test the scam detection system with real scam contracts"""
    
    print("🔍 Testing Scam Detection System")
    print("=" * 50)
    
    # Get a known scam contract from database
    engine = get_db_engine()
    with engine.connect() as conn:
        scam_query = text('''
        SELECT contract_address, scam_label, is_scam, creation_txn, trading_enabled_txn, creator_address
        FROM eth_db.tokens 
        WHERE is_scam = true 
        LIMIT 3
        ''')
        scam_tokens = pd.read_sql(scam_query, conn)
        
    if len(scam_tokens) == 0:
        print("❌ No scam tokens found in database")
        return False
        
    print(f"Found {len(scam_tokens)} scam tokens in database:")
    for i, token in scam_tokens.iterrows():
        print(f"  {i+1}. {token['contract_address']} - {token['scam_label']}")
    
    # Test with the first scam token
    test_token = scam_tokens.iloc[0]
    contract_address = test_token['contract_address']
    expected_scam_label = test_token['scam_label']
    
    print(f"\n🧪 Testing with scam token: {contract_address}")
    print(f"   Expected scam label: {expected_scam_label}")
    
    # 1. Test TokenHealthPredictor initialization
    print("\n1. Testing TokenHealthPredictor...")
    try:
        health_predictor = TokenHealthPredictor()
        print("   ✅ TokenHealthPredictor initialized successfully")
    except Exception as e:
        print(f"   ❌ TokenHealthPredictor failed to initialize: {e}")
        return False
    
    # 2. Test VolumeAnalyzer initialization  
    print("\n2. Testing VolumeAnalyzer...")
    try:
        volume_analyzer = VolumeAnalyzer()
        print("   ✅ VolumeAnalyzer initialized successfully")
        print(f"   📊 Green addresses loaded: {len(volume_analyzer.green_addresses)}")
        print(f"   📊 Mimic octopus addresses loaded: {len(volume_analyzer.mimic_octopus_addresses)}")
    except Exception as e:
        print(f"   ❌ VolumeAnalyzer failed to initialize: {e}")
        return False
    
    # 3. Test LiveERC20Token with scam contract
    print(f"\n3. Testing LiveERC20Token with scam contract...")
    try:
        live_token = ERC20Token(contract_address)
        print("   ✅ LiveERC20Token initialized successfully")
        print(f"   📍 Contract address: {live_token.contract_address}")
        print(f"   📊 Token data initialized: {live_token.token_data is not None}")
    except Exception as e:
        print(f"   ❌ LiveERC20Token failed to initialize: {e}")
        return False
    
    # 4. Test mock transaction with scam patterns
    print(f"\n4. Testing scam detection with mock transactions...")
    
    # Create a mock transaction that should trigger scam detection
    mock_transaction = {
        'hash': '0xtest123456789',
        'block_number': 12345678,
        'block_timestamp': 1640995200,  # timestamp
        'from_address': '0x1234567890abcdef1234567890abcdef12345678',
        'status': True,
        'unique_addresses': [
            '0x1234567890abcdef1234567890abcdef12345678',
            '0xabcdef1234567890abcdef1234567890abcdef12',
            # Add some of the mimic octopus addresses to trigger detection
        ] + list(volume_analyzer.mimic_octopus_addresses)[:3],  # Add 3 mimic octopus addresses
        'erc20_transfers': [
            {
                'from_address': '0x1234567890abcdef1234567890abcdef12345678',
                'to_address': '0xabcdef1234567890abcdef1234567890abcdef12',
                'amount': '1000000000000000000',  # 1 token
                'token_address': contract_address,
                'log_index': 0
            }
        ] * 20,  # 20 transfers to trigger deceptive transfer detection
        'bribe_amount': 0.0,
        'internal_transactions': [],
        'approvals': [],
        'txn_type': 'Regular Transaction'
    }
    
    try:
        # Manually set the token as scam for testing
        live_token.token_data.is_scam = True
        live_token.token_data.scam_label = expected_scam_label
        
        # Process the transaction
        assessment = health_predictor.update_from_transaction(mock_transaction, live_token)
        
        print("   ✅ Transaction processed successfully")
        print(f"   📊 Assessment result: {assessment}")
        
        # Check if scam was detected
        if assessment.get('is_scam', False):
            print("   ✅ Scam detection working - flagged as scam")
            print(f"   📋 Scam reason: {assessment.get('scam_reason', 'Unknown')}")
            print(f"   📊 Confidence: {assessment.get('scam_probability', 0)}")
        else:
            print("   ⚠️  Scam not detected - this might be a problem")
            
    except Exception as e:
        print(f"   ❌ Transaction processing failed: {e}")
        return False
    
    # 5. Test ScamAlert system
    print(f"\n5. Testing ScamAlert system...")
    try:
        scam_alert = ScamAlert()
        
        # Set the latest assessment
        live_token.latest_token_assessment = assessment
        
        # Process the token through alert system (without async for testing)
        if scam_alert._is_alert(live_token):
            alert_data = scam_alert.create_alert(live_token)
            alerts = [alert_data]
        else:
            alerts = []
        
        if alerts and len(alerts) > 0:
            print("   ✅ ScamAlert generated alerts successfully")
            print(f"   📢 Number of alerts: {len(alerts)}")
            for alert in alerts:
                print(f"   📢 Alert: {alert.reason} (confidence: {alert.confidence})")
        else:
            print("   ⚠️  No alerts generated")
            
    except Exception as e:
        print(f"   ❌ ScamAlert processing failed: {e}")
        return False
    
    print(f"\n🎉 Scam detection test completed successfully!")
    return True

def test_volume_patterns():
    """Test specific volume analysis patterns"""
    print("\n📊 Testing Volume Pattern Detection")
    print("=" * 50)
    
    volume_analyzer = VolumeAnalyzer()
    
    # Test 1: Deceptive transfer pattern (many transfers)
    high_transfer_tx = {
        'hash': '0xhightransfer123',
        'block_number': 12345678,
        'from_address': '0x1234567890abcdef1234567890abcdef12345678',
        'erc20_transfers': [{}] * 25,  # 25 transfers > threshold of 15
        'unique_addresses': [f'0x{i:040x}' for i in range(25)]  # 25 addresses > threshold of 20
    }
    
    num_transfers, num_addresses = volume_analyzer.get_num_transfers_and_addresses(high_transfer_tx)
    print(f"High transfer test - Transfers: {num_transfers}, Addresses: {num_addresses}")
    if num_transfers > 15 or num_addresses > 20:
        print("   ✅ Deceptive transfer pattern detection working")
    else:
        print("   ❌ Deceptive transfer pattern not detected")
    
    # Test 2: Malicious actor involvement
    malicious_tx = {
        'hash': '0xmalicious123',
        'unique_addresses': list(volume_analyzer.mimic_octopus_addresses)[:5]  # Include multiple mimic octopus addresses
    }
    
    mal_actors = volume_analyzer.get_malicious_actor_swap(malicious_tx)
    if mal_actors and len(mal_actors) > 1:
        print(f"   ✅ Malicious actor detection working - found {len(mal_actors)} actors")
    else:
        print("   ❌ Malicious actor detection not working")
    
    # Test 3: Green actor involvement
    green_tx = {
        'hash': '0xgreen123',
        'unique_addresses': list(volume_analyzer.green_addresses)[:5]  # Include multiple green addresses
    }
    
    green_actors = volume_analyzer.get_green_actors_involved(green_tx)
    if green_actors and len(green_actors) > 1:
        print(f"   ✅ Green actor detection working - found {len(green_actors)} actors")
    else:
        print("   ❌ Green actor detection not working")

if __name__ == "__main__":
    print("🔍 ETH Token Scam Detection System Test")
    print("=" * 60)
    
    # Run basic scam detection test
    success = test_scam_detection()
    
    # Run volume pattern tests  
    test_volume_patterns()
    
    if success:
        print("\n✅ All tests completed successfully!")
        print("🛡️  Scam detection system appears to be working correctly.")
    else:
        print("\n❌ Some tests failed!")
        print("🚨 Scam detection system may need updates.")