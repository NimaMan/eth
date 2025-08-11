#!/usr/bin/env python3
"""
Test that all fields are properly exposed in Python bindings
"""

import tx_processor_py

reth_datadir = "/home/nima/.local/share/reth/mainnet"
tx_processor = tx_processor_py.TxProcessor(reth_datadir)

# Process the test transaction
tx_hash = "0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7"
processed_tx = tx_processor.process_transaction(tx_hash)

print("\n🔍 Testing All Fields Exposure")
print("=" * 50)

# Test basic fields
print("\n📋 Basic Fields:")
print(f"  hash: {processed_tx.hash}")
print(f"  block_number: {processed_tx.block_number}")
print(f"  block_timestamp: {processed_tx.block_timestamp}")
print(f"  txn_index: {processed_tx.txn_index}")
print(f"  from_address: {processed_tx.from_address}")
print(f"  to_address: {processed_tx.to_address}")
print(f"  value: {processed_tx.value}")
print(f"  status: {processed_tx.status}")
print(f"  nonce: {processed_tx.nonce}")
print(f"  txn_type: {processed_tx.txn_type}")
print(f"  input: {processed_tx.input[:20]}... (length: {len(processed_tx.input)})")
print(f"  bribe_amount: {processed_tx.bribe_amount}")

# Test address sets
print("\n📍 Address Sets:")
print(f"  unique_addresses: {len(processed_tx.unique_addresses)} addresses")
print(f"  erc20_contracts: {len(processed_tx.erc20_contracts)} contracts")

# Test event lists
print("\n📊 Event Lists:")
print(f"  eth_transfers: {len(processed_tx.eth_transfers)}")
print(f"  erc20_transfers: {len(processed_tx.erc20_transfers)}")
print(f"  erc721_transfers: {len(processed_tx.erc721_transfers)}")
print(f"  erc1155_transfers: {len(processed_tx.erc1155_transfers)}")
print(f"  internal_transactions: {len(processed_tx.internal_transactions)}")
print(f"  uniswap_v2_syncs: {len(processed_tx.uniswap_v2_syncs)}")
print(f"  uniswap_v2_swaps: {len(processed_tx.uniswap_v2_swaps)}")
print(f"  approvals: {len(processed_tx.approvals)}")
print(f"  mints: {len(processed_tx.mints)}")
print(f"  burns: {len(processed_tx.burns)}")
print(f"  deposits: {len(processed_tx.deposits)}")
print(f"  withdraws: {len(processed_tx.withdraws)}")

# Test fees structure
print("\n💰 Fees:")
fees = processed_tx.fees
print(f"  gas_price: {fees['gas_price']}")
print(f"  gas_used: {fees['gas_used']}")
print(f"  txn_fee: {fees['txn_fee']}")
print(f"  protocol_type: {fees.get('protocol_type', 'unknown')}")
if 'max_fee_per_gas' in fees:
    print(f"  max_fee_per_gas: {fees['max_fee_per_gas']}")
if 'max_priority_fee' in fees:
    print(f"  max_priority_fee: {fees['max_priority_fee']}")

# Show some ERC20 transfers
if processed_tx.erc20_transfers:
    print("\n🔄 Sample ERC20 Transfer:")
    transfer = processed_tx.erc20_transfers[0]
    print(f"  token: {transfer['token_address']}")
    print(f"  from: {transfer['from_address']}")
    print(f"  to: {transfer['to_address']}")
    print(f"  amount: {transfer['amount']}")

# Show burns if any
if processed_tx.burns:
    print("\n🔥 Burns Detected:")
    for burn in processed_tx.burns:
        print(f"  pair: {burn['pair_address']}")
        print(f"  sender: {burn['sender']}")
        print(f"  amount: {burn['amount']}")

# Test to_dict conversion
print("\n📦 Testing to_dict():")
tx_dict = processed_tx.to_dict()
print(f"  Dictionary keys: {len(tx_dict)} fields")

# Check for all expected fields
expected_fields = [
    'hash', 'block_number', 'block_timestamp', 'txn_index',
    'from_address', 'to_address', 'value', 'status', 'nonce',
    'txn_type', 'actions', 'input', 'bribe_amount',
    'unique_addresses', 'erc20_contracts', 
    'eth_transfers', 'erc20_transfers', 'erc721_transfers', 'erc1155_transfers',
    'internal_transactions', 'uniswap_v2_syncs', 'uniswap_v2_swaps',
    'approvals', 'mints', 'burns', 'deposits', 'withdraws',
    'pair_events', 'owner_events', 'contract_creation_events',
    'trading_enabled_events', 'trading_disabled_events',
    'uniswap_v3_pools', 'uniswap_v3_swaps',
    'other_events', 'state_changes', 'latest_states', 'fees'
]

missing_fields = [f for f in expected_fields if f not in tx_dict]
if missing_fields:
    print(f"\n⚠️ Missing fields: {missing_fields}")
else:
    print("\n✅ All expected fields present!")

print("\n" + "=" * 50)
print("✅ Field exposure test complete!")