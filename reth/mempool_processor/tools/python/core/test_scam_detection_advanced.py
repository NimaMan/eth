#!/usr/bin/env python3
"""
Advanced Scam Detection Test

Tests the scam detection system with real blockchain data and edge cases
to ensure it's robust and up-to-date.
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
from datetime import datetime

def test_real_transaction_data():
    """Test with real transaction data from database"""
    print("🔍 Testing with Real Transaction Data")
    print("=" * 50)
    
    engine = get_db_engine()
    
    # Get some recent transaction data that involves tokens
    with engine.connect() as conn:
        # Look for trades involving scam tokens
        trade_query = text('''
        SELECT t.token_address, tr.txn_hash, tr.entry_block, tr.amount_out, tr.amount_in,
               tk.scam_label, tk.is_scam
        FROM eth_db.trades tr
        JOIN eth_db.tokens tk ON tr.token_address = tk.contract_address  
        WHERE tk.is_scam = true
        ORDER BY tr.entry_block DESC
        LIMIT 5
        ''')
        
        try:
            trades = pd.read_sql(trade_query, conn)
            print(f"Found {len(trades)} recent trades involving scam tokens")
            
            if len(trades) > 0:
                print("Recent scam token trades:")
                for i, trade in trades.iterrows():
                    print(f"  Token: {trade['token_address'][:10]}... | Block: {trade['entry_block']} | Scam: {trade['scam_label']}")
                    
                # Test with the first trade
                test_trade = trades.iloc[0]
                token_address = test_trade['token_address']
                
                print(f"\n🧪 Testing detection with real trade data:")
                print(f"   Token: {token_address}")
                print(f"   Block: {test_trade['entry_block']}")
                print(f"   Known scam label: {test_trade['scam_label']}")
                
                # Create LiveERC20Token for this address
                live_token = ERC20Token(token_address)
                
                # Simulate the real transaction
                real_transaction = {
                    'hash': test_trade['txn_hash'],
                    'block_number': int(test_trade['entry_block']),
                    'block_timestamp': datetime.now(),
                    'from_address': '0x' + '1' * 40,  # Mock address
                    'status': True,
                    'unique_addresses': ['0x' + '1' * 40, '0x' + '2' * 40],
                    'erc20_transfers': [],
                    'bribe_amount': 0.0,
                    'internal_transactions': [],
                    'approvals': [],
                    'txn_type': 'Regular Transaction'
                }
                
                # Set the known scam status
                live_token.token_data.is_scam = bool(test_trade['is_scam'])
                live_token.token_data.scam_label = test_trade['scam_label']
                
                # Test the detection
                health_predictor = TokenHealthPredictor()
                assessment = health_predictor.update_from_transaction(real_transaction, live_token)
                
                if assessment.get('is_scam', False):
                    print("   ✅ Real transaction scam detection working")
                    print(f"   📋 Detected reason: {assessment.get('scam_reason', 'Unknown')}")
                else:
                    print("   ⚠️  Real transaction scam not detected")
                
            else:
                print("   ⚠️  No recent scam token trades found in database")
                
        except Exception as e:
            print(f"   ❌ Error querying trades: {e}")

def test_edge_cases():
    """Test edge cases and potential issues"""
    print("\n🧪 Testing Edge Cases")
    print("=" * 50)
    
    volume_analyzer = VolumeAnalyzer()
    health_predictor = TokenHealthPredictor()
    
    # Test 1: Empty transaction
    print("1. Testing empty transaction...")
    empty_tx = {
        'hash': '0xempty',
        'block_number': 12345678,
        'from_address': '0x' + '0' * 40,
        'status': True,
        'unique_addresses': [],
        'erc20_transfers': [],
        'bribe_amount': 0.0,
        'internal_transactions': [],
        'approvals': [],
        'txn_type': 'Regular Transaction'
    }
    
    try:
        live_token = ERC20Token('0x' + '1' * 40)
        assessment = health_predictor.update_from_transaction(empty_tx, live_token)
        print("   ✅ Empty transaction handled successfully")
    except Exception as e:
        print(f"   ❌ Empty transaction failed: {e}")
    
    # Test 2: Transaction with exactly threshold values
    print("\n2. Testing threshold boundary conditions...")
    threshold_tx = {
        'hash': '0xthreshold',
        'block_number': 12345678,
        'from_address': '0x' + '1' * 40,
        'status': True,
        'unique_addresses': [f'0x{i:040x}' for i in range(20)],  # Exactly 20 addresses (threshold)
        'erc20_transfers': [{}] * 15,  # Exactly 15 transfers (threshold)
        'bribe_amount': 0.0,
        'internal_transactions': [],
        'approvals': [],
        'txn_type': 'Regular Transaction'
    }
    
    try:
        assessment = health_predictor.update_from_transaction(threshold_tx, live_token)
        # Should not trigger since it's exactly at threshold, not above
        if not assessment.get('is_scam', False):
            print("   ✅ Threshold boundary handling correct")
        else:
            print("   ⚠️  Threshold boundary may be too sensitive")
    except Exception as e:
        print(f"   ❌ Threshold test failed: {e}")
    
    # Test 3: Transaction just above threshold
    print("\n3. Testing just above threshold...")
    above_threshold_tx = {
        'hash': '0xabove',
        'block_number': 12345678,
        'from_address': '0x' + '1' * 40,
        'status': True,
        'unique_addresses': [f'0x{i:040x}' for i in range(21)],  # 21 addresses (above threshold)
        'erc20_transfers': [{}] * 16,  # 16 transfers (above threshold)
        'bribe_amount': 0.0,
        'internal_transactions': [],
        'approvals': [],
        'txn_type': 'Regular Transaction'
    }
    
    try:
        assessment = health_predictor.update_from_transaction(above_threshold_tx, live_token)
        if assessment.get('is_scam', False):
            print("   ✅ Above threshold detection working")
        else:
            print("   ❌ Above threshold not detected")
    except Exception as e:
        print(f"   ❌ Above threshold test failed: {e}")

def test_address_list_freshness():
    """Test if the address lists are up to date"""
    print("\n📋 Testing Address List Freshness")
    print("=" * 50)
    
    volume_analyzer = VolumeAnalyzer()
    
    print(f"Green addresses (whales + orca): {len(volume_analyzer.green_addresses)}")
    print(f"Mimic octopus addresses (from DB): {len(volume_analyzer.mimic_octopus_addresses)}")
    
    # Check if we have a reasonable number of addresses
    if len(volume_analyzer.green_addresses) < 50:
        print("   ⚠️  Green address list seems small - may need updating")
    else:
        print("   ✅ Green address list size looks reasonable")
    
    if len(volume_analyzer.mimic_octopus_addresses) < 1000:
        print("   ⚠️  Mimic octopus list seems small - may need updating")
    else:
        print("   ✅ Mimic octopus list size looks reasonable")
    
    # Sample a few addresses to check format
    sample_green = list(volume_analyzer.green_addresses)[:3]
    sample_mimic = list(volume_analyzer.mimic_octopus_addresses)[:3]
    
    print(f"\nSample green addresses: {sample_green}")
    print(f"Sample mimic addresses: {sample_mimic}")
    
    # Check address format
    for addr in sample_green + sample_mimic:
        if not addr.startswith('0x') or len(addr) != 42:
            print(f"   ⚠️  Invalid address format: {addr}")
            break
    else:
        print("   ✅ Address formats look correct")

def test_database_connectivity():
    """Test database connectivity and data freshness"""
    print("\n🗄️  Testing Database Connectivity")
    print("=" * 50)
    
    try:
        engine = get_db_engine()
        with engine.connect() as conn:
            # Check recent data
            recent_query = text('''
            SELECT COUNT(*) as count, MAX(creation_txn) as latest_txn
            FROM eth_db.tokens
            WHERE is_scam = true
            ''')
            result = conn.execute(recent_query).fetchone()
            
            print(f"Total scam tokens in DB: {result[0]}")
            print(f"Latest scam txn: {result[1]}")
            
            if result[0] > 100:
                print("   ✅ Good amount of scam data in database")
            else:
                print("   ⚠️  Limited scam data - database may need updates")
                
    except Exception as e:
        print(f"   ❌ Database connectivity issue: {e}")

if __name__ == "__main__":
    print("🧪 Advanced Scam Detection System Test")
    print("=" * 60)
    
    # Run all advanced tests
    test_real_transaction_data()
    test_edge_cases()
    test_address_list_freshness()
    test_database_connectivity()
    
    print("\n🏁 Advanced tests completed!")
    print("💡 Review any warnings above to ensure optimal scam detection performance.")