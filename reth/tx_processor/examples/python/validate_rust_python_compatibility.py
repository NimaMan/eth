#!/usr/bin/env python3
"""
Validate Compatibility: Rust tx_processor vs Python eth_data

This script compares the outputs of both implementations to verify they produce
identical results for the same transactions.
"""

import json
import time
from typing import Dict, Any, List
from web3 import Web3
from deepdiff import DeepDiff

import rs_tx_processor
from eth_data.txn.txn_processor import TransactionProcessor
from eth_data.txn.txn_data_fetcher import TransactionDataFetcher


def normalize_value(value: Any) -> str:
    """Normalize numeric values to string for comparison"""
    if value is None:
        return "0"
    if isinstance(value, (int, float)):
        return str(int(value))
    return str(value)

def normalize_processed_tx(tx_dict: Dict) -> Dict:
    """Normalize a processed transaction dictionary for comparison"""
    normalized = {}
    
    # Keep addresses as-is (they should already be checksum)
    address_fields = ['from_address', 'to_address', 'contract_address']
    for field in address_fields:
        if field in tx_dict:
            normalized[field] = tx_dict[field]
    
    # Normalize numeric values
    numeric_fields = ['value', 'gas_price', 'gas_used', 'txn_fee', 'bribe_amount']
    for field in numeric_fields:
        if field in tx_dict:
            normalized[field] = normalize_value(tx_dict[field])
    
    # Normalize fees structure
    if 'fees' in tx_dict:
        if isinstance(tx_dict['fees'], dict):
            normalized['fees'] = {
                'gas_price': normalize_value(tx_dict['fees'].get('gas_price')),
                'gas_used': normalize_value(tx_dict['fees'].get('gas_used')),
                'txn_fee': normalize_value(tx_dict['fees'].get('txn_fee'))
            }
    
    # Normalize event lists - ensure addresses are checksummed
    event_fields = ['erc20_transfers', 'internal_transactions', 'approvals', 
                    'uniswap_v2_swaps', 'uniswap_v2_syncs', 'mints', 'burns']
    
    for field in event_fields:
        if field in tx_dict and isinstance(tx_dict[field], list):
            normalized[field] = []
            for event in tx_dict[field]:
                if isinstance(event, dict):
                    norm_event = {}
                    for k, v in event.items():
                        if 'address' in k.lower():
                            norm_event[k] = v  # Keep addresses as-is (should be checksum)
                        elif k in ['amount', 'value', 'reserve0', 'reserve1']:
                            norm_event[k] = normalize_value(v)
                        else:
                            norm_event[k] = v
                    normalized[field].append(norm_event)
                else:
                    normalized[field].append(event)
    
    # Copy other fields as-is
    for field in ['hash', 'block_number', 'block_timestamp', 'txn_index', 
                  'status', 'nonce', 'txn_type', 'input', 'actions']:
        if field in tx_dict:
            normalized[field] = tx_dict[field]
    
    # Handle unique_addresses and erc20_contracts (sets)
    if 'unique_addresses' in tx_dict:
        addrs = tx_dict['unique_addresses']
        if isinstance(addrs, (set, list)):
            normalized['unique_addresses'] = sorted([a for a in addrs if a])
    
    if 'erc20_contracts' in tx_dict:
        contracts = tx_dict['erc20_contracts']
        if isinstance(contracts, (set, list)):
            normalized['erc20_contracts'] = sorted([a for a in contracts if a])
    
    return normalized

def process_with_python(tx_hash: str, w3: Web3) -> Dict:
    """Process transaction with Python implementation"""
    processor = TransactionProcessor(w3=w3, calculate_state_changes=False)
    fetcher = TransactionDataFetcher(w3=w3)
    
    # Fetch transaction data
    transaction = w3.eth.get_transaction(tx_hash)
    receipt = w3.eth.get_transaction_receipt(tx_hash)
    
    # Get trace if needed
    trace = None
    if processor.needs_trace(transaction):
        try:
            trace = w3.manager.request_blocking("debug_traceTransaction", [tx_hash, {"tracer": "callTracer"}])
        except:
            pass
    
    # Process transaction
    processed = processor.process_transaction(
        dict(transaction),
        dict(receipt),
        trace,
        block_timestamp=w3.eth.get_block(transaction['blockNumber'])['timestamp']
    )
    
    # Convert to dict
    return processed.to_dict() if hasattr(processed, 'to_dict') else processed.__dict__

def process_with_rust(tx_hash: str) -> Dict:
    """Process transaction with Rust implementation"""
    processor = rs_tx_processor.TxProcessor()
    processed = processor.process_transaction(tx_hash)
    return processed.to_dict()

def compare_implementations(tx_hash: str) -> Dict[str, Any]:
    """Compare both implementations for a given transaction"""
    print(f"\n{'='*80}")
    print(f"Comparing transaction: {tx_hash}")
    print(f"{'='*80}")
    
    # Initialize Web3 for Python implementation
    w3 = Web3(Web3.HTTPProvider('http://localhost:8545'))
    
    # Process with Python
    print("\n📊 Processing with Python implementation...")
    py_start = time.time()
    try:
        python_result = process_with_python(tx_hash, w3)
        py_time = time.time() - py_start
        print(f"   ✅ Python processing completed in {py_time:.3f}s")
        python_normalized = normalize_processed_tx(python_result)
    except Exception as e:
        print(f"   ❌ Python processing failed: {e}")
        return {"error": f"Python failed: {e}"}
    
    # Process with Rust
    print("\n🦀 Processing with Rust implementation...")
    rust_start = time.time()
    try:
        rust_result = process_with_rust(tx_hash)
        rust_time = time.time() - rust_start
        print(f"   ✅ Rust processing completed in {rust_time:.3f}s")
        rust_normalized = normalize_processed_tx(rust_result)
    except Exception as e:
        print(f"   ❌ Rust processing failed: {e}")
        return {"error": f"Rust failed: {e}"}
    
    # Performance comparison
    speedup = py_time / rust_time if rust_time > 0 else 0
    print(f"\n⚡ Performance: Rust is {speedup:.1f}x faster")
    
    # Compare results
    print("\n🔍 Comparing outputs...")
    
    # Key fields to compare
    comparison_results = {
        "tx_hash": tx_hash,
        "python_time": py_time,
        "rust_time": rust_time,
        "speedup": speedup,
        "matches": {},
        "differences": {}
    }
    
    # Compare basic fields
    basic_fields = ['hash', 'block_number', 'block_timestamp', 'from_address', 
                    'to_address', 'value', 'status', 'nonce', 'txn_type']
    
    for field in basic_fields:
        py_val = python_normalized.get(field)
        rust_val = rust_normalized.get(field)
        if py_val == rust_val:
            comparison_results["matches"][field] = True
            print(f"   ✅ {field}: Match")
        else:
            comparison_results["differences"][field] = {
                "python": py_val,
                "rust": rust_val
            }
            print(f"   ❌ {field}: Mismatch")
            print(f"      Python: {py_val}")
            print(f"      Rust:   {rust_val}")
    
    # Compare event counts
    event_fields = ['erc20_transfers', 'internal_transactions', 'approvals',
                    'uniswap_v2_swaps', 'uniswap_v2_syncs', 'mints', 'burns']
    
    print("\n📊 Event Counts:")
    for field in event_fields:
        py_count = len(python_normalized.get(field, []))
        rust_count = len(rust_normalized.get(field, []))
        if py_count == rust_count:
            print(f"   ✅ {field}: {py_count} (match)")
            comparison_results["matches"][f"{field}_count"] = py_count
        else:
            print(f"   ❌ {field}: Python={py_count}, Rust={rust_count}")
            comparison_results["differences"][f"{field}_count"] = {
                "python": py_count,
                "rust": rust_count
            }
    
    # Deep comparison
    diff = DeepDiff(python_normalized, rust_normalized, ignore_order=True)
    if not diff:
        print("\n✅ PERFECT MATCH: Both implementations produce identical results!")
        comparison_results["identical"] = True
    else:
        print("\n⚠️ Differences found:")
        comparison_results["identical"] = False
        comparison_results["deep_diff"] = str(diff)
        print(json.dumps(diff, indent=2, default=str))
    
    return comparison_results

def main():
    """Run comprehensive audit"""
    print("🔍 TX Processor Implementation Audit")
    print("=" * 80)
    print("Comparing Rust tx_processor vs Python eth_data")
    print("=" * 80)
    
    # Test transactions covering different scenarios
    test_transactions = [
        # Complex DeFi swap with multiple events
        "0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7",
        
        # Simple ETH transfer (no logs, no simulation needed)
        # "0x[simple_eth_transfer_hash]",
        
        # ERC20 transfer
        # "0x[erc20_transfer_hash]",
        
        # Contract creation
        # "0x[contract_creation_hash]",
    ]
    
    results = []
    for tx_hash in test_transactions:
        try:
            result = compare_implementations(tx_hash)
            results.append(result)
        except Exception as e:
            print(f"\n❌ Error processing {tx_hash}: {e}")
            results.append({"tx_hash": tx_hash, "error": str(e)})
    
    # Summary
    print("\n" + "=" * 80)
    print("AUDIT SUMMARY")
    print("=" * 80)
    
    total = len(results)
    perfect_matches = sum(1 for r in results if r.get("identical", False))
    errors = sum(1 for r in results if "error" in r)
    
    print(f"\nTotal transactions tested: {total}")
    print(f"Perfect matches: {perfect_matches}")
    print(f"Errors: {errors}")
    
    if perfect_matches == total:
        print("\n✅ SUCCESS: All transactions match perfectly!")
    else:
        print(f"\n⚠️ WARNING: {total - perfect_matches} transactions have differences")
    
    # Save detailed results
    with open("audit_results.json", "w") as f:
        json.dump(results, f, indent=2, default=str)
    print("\n📄 Detailed results saved to audit_results.json")

if __name__ == "__main__":
    main()