#!/usr/bin/env python3
"""
Test script to verify tx_processor_py works with all fields in Jupyter
"""

import tx_processor_py
import sys

print(f"Python executable: {sys.executable}")
print(f"Python version: {sys.version}")

# Initialize processor
reth_datadir = "/home/nima/.local/share/reth/mainnet"
tx_processor = tx_processor_py.TxProcessor(reth_datadir)

# Process transaction
tx_hash = "0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7"
processed_tx = tx_processor.process_transaction(tx_hash)

# Display all fields like the Python ProcessedTransaction would
print(f"\nProcessedTransaction(")
print(f"  hash='{processed_tx.hash}',")
print(f"  block_number={processed_tx.block_number},")
print(f"  block_timestamp={processed_tx.block_timestamp},")
print(f"  txn_index={processed_tx.txn_index},")
print(f"  from_address='{processed_tx.from_address}',")
print(f"  to_address='{processed_tx.to_address}',")
print(f"  contract_address={processed_tx.contract_address},")
print(f"  value={processed_tx.value},")
print(f"  status={processed_tx.status},")
print(f"  nonce={processed_tx.nonce},")
print(f"  txn_type='{processed_tx.txn_type}',")
print(f"  actions={processed_tx.actions},")
print(f"  fees={processed_tx.fees},")
print(f"  bribe_amount={processed_tx.bribe_amount},")
print(f"  unique_addresses={{{', '.join([f"'{a}'" for a in list(processed_tx.unique_addresses)[:3]])}...}} ({len(processed_tx.unique_addresses)} total),")
print(f"  erc20_contracts={{{', '.join([f"'{a}'" for a in list(processed_tx.erc20_contracts)[:3]])}...}} ({len(processed_tx.erc20_contracts)} total),")
print(f"  eth_transfers={processed_tx.eth_transfers},")
print(f"  erc20_transfers=[...] ({len(processed_tx.erc20_transfers)} transfers),")
print(f"  erc721_transfers={processed_tx.erc721_transfers},")
print(f"  erc1155_transfers={processed_tx.erc1155_transfers},")
print(f"  internal_transactions=[...] ({len(processed_tx.internal_transactions)} transactions),")
print(f"  uniswap_v2_syncs=[...] ({len(processed_tx.uniswap_v2_syncs)} syncs),")
print(f"  uniswap_v2_swaps={processed_tx.uniswap_v2_swaps},")
print(f"  approvals={processed_tx.approvals},")
print(f"  mints={processed_tx.mints},")
print(f"  burns={processed_tx.burns},")
print(f"  deposits={processed_tx.deposits},")
print(f"  withdraws={processed_tx.withdraws},")
print(f"  pair_events={processed_tx.pair_events},")
print(f"  owner_events={processed_tx.owner_events},")
print(f"  contract_creation_events={processed_tx.contract_creation_events},")
print(f"  trading_enabled_events={processed_tx.trading_enabled_events},")
print(f"  trading_disabled_events={processed_tx.trading_disabled_events},")
print(f"  uniswap_v3_pools={processed_tx.uniswap_v3_pools},")
print(f"  uniswap_v3_swaps={processed_tx.uniswap_v3_swaps},")
print(f"  other_events={processed_tx.other_events},")
print(f"  state_changes={{{len(processed_tx.state_changes)} changes}},")
print(f"  latest_states={processed_tx.latest_states},")
print(f"  input='{processed_tx.input[:50]}...'")
print(f")")

print(f"\n✅ All fields are accessible!")