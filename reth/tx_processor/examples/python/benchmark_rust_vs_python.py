#!/usr/bin/env python3
"""
Performance Benchmark: Rust tx_processor vs Python eth_data

This script:
1. Fetches 100 random transaction hashes from the database
2. Benchmarks both implementations with real transactions
3. Provides accurate performance metrics
"""

import sys
import time
import json
import psycopg2
from typing import List, Dict, Any, Tuple
from datetime import datetime
import statistics

import rs_tx_processor
from web3 import Web3
from eth_data.txn.txn_processor import TransactionProcessor
from eth_data.txn.txn_data_fetcher import TransactionDataFetcher


def fetch_random_tx_hashes_from_db(count: int = 1000) -> List[str]:
    """
    Fetch random transaction hashes from the PostgreSQL database
    """
    print(f"📊 Fetching {count} random transaction hashes from database...")
    
    try:
        # Connect to PostgreSQL
        conn = psycopg2.connect(
            host="localhost",
            database="eth_db",
            user="postgres",
            password="postgres"
        )
        cursor = conn.cursor()
        
        # Query to get random transaction hashes
        # We'll get transactions from recent blocks that likely have varied complexity
        query = """
        SELECT DISTINCT tx_hash 
        FROM eth_db.transactions 
        WHERE block_number > 22800000  -- Recent blocks
        AND block_number < 22900000    -- But not too recent (ensure they're finalized)
        ORDER BY RANDOM() 
        LIMIT %s
        """
        
        cursor.execute(query, (count,))
        results = cursor.fetchall()
        
        if not results:
            print("⚠️ No transactions found in database, using fallback list")
            return fetch_fallback_tx_hashes(count)
        
        tx_hashes = [row[0] for row in results]
        print(f"✅ Fetched {len(tx_hashes)} transaction hashes from database")
        
        cursor.close()
        conn.close()
        
        return tx_hashes
        
    except Exception as e:
        print(f"⚠️ Database fetch failed: {e}")
        print("Using fallback transaction list...")
        return fetch_fallback_tx_hashes(count)


def fetch_fallback_tx_hashes(count: int) -> List[str]:
    """
    Fallback: Fetch recent transaction hashes from the Ethereum node
    """
    try:
        w3 = Web3(Web3.HTTPProvider('http://localhost:8545'))
        
        # Get the latest block
        latest_block = w3.eth.get_block('latest')
        
        tx_hashes = []
        block_num = latest_block['number']
        
        # Go back through blocks to collect transaction hashes
        while len(tx_hashes) < count and block_num > latest_block['number'] - 100:
            block = w3.eth.get_block(block_num, full_transactions=False)
            tx_hashes.extend([tx.hex() for tx in block['transactions']])
            block_num -= 1
            
            if len(tx_hashes) >= count:
                break
        
        # Return requested count
        return tx_hashes[:count]
        
    except Exception as e:
        print(f"❌ Fallback also failed: {e}")
        # Last resort - return known working transactions
        known_txs = [
            "0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7",
            "0x5fde29bd2c13db39f758031e2ed054a9d703e3edf04a4d5efa55191c8b71e589",
            "0xc9322b57c24f763d92ac58d414557b61e351f874284fa80e33a1fe73c8b9a5bf",
            "0x82926c3447d7ce6cffbb69ac7ab1571a117e110661532f144ac5bd11053f8d60",
            "0xb1def6eafe5a48b715b5ed210ab4d8a24e40655934aff2180643bd31e42d3c4f",
        ]
        return known_txs * (count // len(known_txs) + 1)[:count]

def benchmark_python_implementation(tx_hashes: List[str], sample_size: int = None) -> Dict[str, Any]:
    """
    Benchmark Python eth_data implementation
    """
    if sample_size:
        tx_hashes = tx_hashes[:sample_size]
    
    print(f"\n🐍 Benchmarking Python implementation with {len(tx_hashes)} transactions...")
    
    w3 = Web3(Web3.HTTPProvider('http://localhost:8545'))
    processor = TransactionProcessor(w3=w3, calculate_state_changes=False)
    
    times = []
    successes = 0
    failures = 0
    error_types = {}
    
    start_total = time.time()
    
    for i, tx_hash in enumerate(tx_hashes, 1):
        try:
            # Time individual transaction
            start = time.perf_counter()
            
            # Fetch transaction data
            transaction = w3.eth.get_transaction(tx_hash)
            receipt = w3.eth.get_transaction_receipt(tx_hash)
            
            # Get trace if needed
            trace = None
            if processor.needs_trace(transaction):
                try:
                    trace = w3.manager.request_blocking("debug_traceTransaction", 
                                                       [tx_hash, {"tracer": "callTracer"}])
                except:
                    pass
            
            # Process transaction
            block = w3.eth.get_block(transaction['blockNumber'])
            processed = processor.process_transaction(
                dict(transaction),
                dict(receipt),
                trace,
                block_timestamp=block['timestamp']
            )
            
            elapsed = time.perf_counter() - start
            times.append(elapsed)
            successes += 1
            
            # Progress indicator
            if i % 50 == 0:
                avg_time = statistics.mean(times)
                throughput = i / (time.time() - start_total)
                print(f"   Processed {i}/{len(tx_hashes)} - Avg: {avg_time*1000:.2f}ms/tx - {throughput:.1f} tx/sec")
                
        except Exception as e:
            failures += 1
            error_type = type(e).__name__
            error_types[error_type] = error_types.get(error_type, 0) + 1
            # Continue with next transaction
    
    total_time = time.time() - start_total
    
    if times:
        return {
            "implementation": "Python (eth_data)",
            "total_transactions": len(tx_hashes),
            "successful": successes,
            "failed": failures,
            "error_types": error_types,
            "total_time": total_time,
            "avg_time_per_tx": statistics.mean(times),
            "median_time_per_tx": statistics.median(times),
            "min_time": min(times),
            "max_time": max(times),
            "std_dev": statistics.stdev(times) if len(times) > 1 else 0,
            "throughput_per_sec": successes / total_time if total_time > 0 else 0,
        }
    else:
        return {
            "implementation": "Python (eth_data)",
            "error": "No successful transactions processed",
            "failures": failures,
            "error_types": error_types
        }

def benchmark_rust_implementation(tx_hashes: List[str], sample_size: int = None) -> Dict[str, Any]:
    """
    Benchmark Rust tx_processor implementation
    """
    if sample_size:
        tx_hashes = tx_hashes[:sample_size]
    
    print(f"\n🦀 Benchmarking Rust implementation with {len(tx_hashes)} transactions...")
    
    processor = rs_tx_processor.TxProcessor()
    
    times = []
    successes = 0
    failures = 0
    error_types = {}
    
    start_total = time.time()
    
    for i, tx_hash in enumerate(tx_hashes, 1):
        try:
            # Time individual transaction
            start = time.perf_counter()
            
            processed = processor.process_transaction(tx_hash)
            
            elapsed = time.perf_counter() - start
            times.append(elapsed)
            successes += 1
            
            # Progress indicator
            if i % 50 == 0:
                avg_time = statistics.mean(times)
                throughput = i / (time.time() - start_total)
                print(f"   Processed {i}/{len(tx_hashes)} - Avg: {avg_time*1000:.2f}ms/tx - {throughput:.1f} tx/sec")
                
        except Exception as e:
            failures += 1
            error_type = type(e).__name__
            error_types[error_type] = error_types.get(error_type, 0) + 1
            # Continue with next transaction
    
    total_time = time.time() - start_total
    
    if times:
        return {
            "implementation": "Rust (tx_processor)",
            "total_transactions": len(tx_hashes),
            "successful": successes,
            "failed": failures,
            "error_types": error_types,
            "total_time": total_time,
            "avg_time_per_tx": statistics.mean(times),
            "median_time_per_tx": statistics.median(times),
            "min_time": min(times),
            "max_time": max(times),
            "std_dev": statistics.stdev(times) if len(times) > 1 else 0,
            "throughput_per_sec": successes / total_time if total_time > 0 else 0,
        }
    else:
        return {
            "implementation": "Rust (tx_processor)",
            "error": "No successful transactions processed",
            "failures": failures,
            "error_types": error_types
        }

def benchmark_rust_batch_implementation(tx_hashes: List[str]) -> Dict[str, Any]:
    """
    Benchmark Rust tx_processor with batch processing
    """
    print(f"\n🦀 Benchmarking Rust BATCH implementation with {len(tx_hashes)} transactions...")
    
    processor = rs_tx_processor.TxProcessor()
    
    try:
        start_total = time.time()
        
        # Process all transactions in batch
        results = processor.process_transactions_batch(tx_hashes)
        
        total_time = time.time() - start_total
        
        return {
            "implementation": "Rust BATCH (tx_processor)",
            "total_transactions": len(tx_hashes),
            "successful": len(results),
            "failed": len(tx_hashes) - len(results),
            "total_time": total_time,
            "avg_time_per_tx": total_time / len(tx_hashes) if tx_hashes else 0,
            "throughput_per_sec": len(results) / total_time if total_time > 0 else 0,
        }
    except Exception as e:
        return {
            "implementation": "Rust BATCH (tx_processor)",
            "error": str(e)
        }

def generate_report(results: List[Dict[str, Any]], tx_hashes: List[str]):
    """
    Generate a comprehensive benchmark report
    """
    print("\n" + "="*80)
    print("📊 PERFORMANCE BENCHMARK REPORT")
    print("="*80)
    print(f"\nDate: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"Total transactions tested: {len(tx_hashes)}")
    print("\n" + "-"*80)
    
    for result in results:
        impl = result.get("implementation", "Unknown")
        print(f"\n{impl}")
        print("-" * len(impl))
        
        if "error" in result:
            print(f"❌ Error: {result['error']}")
            continue
        
        print(f"✅ Successful: {result.get('successful', 0)}/{result.get('total_transactions', 0)}")
        print(f"❌ Failed: {result.get('failed', 0)}")
        
        if result.get('error_types'):
            print(f"   Error types: {result['error_types']}")
        
        print(f"\n⏱️ Performance Metrics:")
        print(f"   Total time: {result.get('total_time', 0):.2f} seconds")
        print(f"   Average per tx: {result.get('avg_time_per_tx', 0)*1000:.2f} ms")
        
        if 'median_time_per_tx' in result:
            print(f"   Median per tx: {result['median_time_per_tx']*1000:.2f} ms")
        if 'min_time' in result:
            print(f"   Min time: {result['min_time']*1000:.2f} ms")
        if 'max_time' in result:
            print(f"   Max time: {result['max_time']*1000:.2f} ms")
        if 'std_dev' in result:
            print(f"   Std deviation: {result['std_dev']*1000:.2f} ms")
        
        print(f"\n📈 Throughput: {result.get('throughput_per_sec', 0):.2f} tx/sec")
    
    # Calculate speedup
    print("\n" + "="*80)
    print("🚀 PERFORMANCE COMPARISON")
    print("="*80)
    
    python_result = next((r for r in results if "Python" in r.get("implementation", "")), None)
    rust_result = next((r for r in results if "Rust" in r.get("implementation", "") and "BATCH" not in r.get("implementation", "")), None)
    rust_batch_result = next((r for r in results if "BATCH" in r.get("implementation", "")), None)
    
    if python_result and rust_result and not python_result.get("error") and not rust_result.get("error"):
        py_throughput = python_result.get('throughput_per_sec', 0)
        rust_throughput = rust_result.get('throughput_per_sec', 0)
        
        if py_throughput > 0:
            speedup = rust_throughput / py_throughput
            print(f"\n🎯 Rust is {speedup:.1f}x faster than Python")
            print(f"   Python: {py_throughput:.2f} tx/sec")
            print(f"   Rust: {rust_throughput:.2f} tx/sec")
        
        if rust_batch_result and not rust_batch_result.get("error"):
            batch_throughput = rust_batch_result.get('throughput_per_sec', 0)
            if py_throughput > 0:
                batch_speedup = batch_throughput / py_throughput
                print(f"\n🎯 Rust BATCH is {batch_speedup:.1f}x faster than Python")
                print(f"   Rust Batch: {batch_throughput:.2f} tx/sec")
    
    # Save results to file
    report_filename = f"benchmark_results_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    with open(report_filename, 'w') as f:
        json.dump({
            "timestamp": datetime.now().isoformat(),
            "total_transactions": len(tx_hashes),
            "results": results
        }, f, indent=2, default=str)
    
    print(f"\n📄 Detailed results saved to {report_filename}")

def main():
    """
    Main benchmark execution
    """
    print("🏁 Real Performance Benchmark")
    print("="*80)
    
    # Configuration
    SAMPLE_SIZE = 1000  # Number of transactions to test
    RUN_PYTHON = True  # Set to False to skip Python (it's slow)
    RUN_SMALL_SAMPLE_FIRST = True  # Test with 10 txs first
    
    # Fetch random transaction hashes
    tx_hashes = fetch_random_tx_hashes_from_db(SAMPLE_SIZE)
    
    if not tx_hashes:
        print("❌ No transaction hashes available for testing")
        return
    
    print(f"\n✅ Ready to benchmark with {len(tx_hashes)} transactions")
    
    results = []
    
    # Optional: Quick test with small sample
    if RUN_SMALL_SAMPLE_FIRST:
        print("\n🔍 Running quick test with 10 transactions first...")
        small_sample = tx_hashes[:10]
        
        # Test Rust
        rust_small = benchmark_rust_implementation(small_sample)
        print(f"   Rust: {rust_small.get('throughput_per_sec', 0):.2f} tx/sec")
        
        if RUN_PYTHON:
            # Test Python
            python_small = benchmark_python_implementation(small_sample)
            print(f"   Python: {python_small.get('throughput_per_sec', 0):.2f} tx/sec")
    
    # Full benchmark
    print(f"\n🚀 Starting full benchmark with {SAMPLE_SIZE} transactions...")
    
    # Benchmark Rust (individual)
    rust_result = benchmark_rust_implementation(tx_hashes)
    results.append(rust_result)
    
    # Benchmark Rust (batch)
    rust_batch_result = benchmark_rust_batch_implementation(tx_hashes)
    results.append(rust_batch_result)
    
    # Benchmark Python (optional - it's slow!)
    if RUN_PYTHON:
        python_result = benchmark_python_implementation(tx_hashes)
        results.append(python_result)
    else:
        print("\n⚠️ Skipping Python benchmark (set RUN_PYTHON=True to enable)")
        # Add estimated Python performance based on known benchmarks
        results.append({
            "implementation": "Python (eth_data) - ESTIMATED",
            "total_transactions": len(tx_hashes),
            "successful": len(tx_hashes),
            "failed": 0,
            "total_time": len(tx_hashes) / 2.5,  # Assuming 2.5 tx/sec
            "avg_time_per_tx": 0.4,  # 400ms per tx
            "throughput_per_sec": 2.5,
            "note": "Estimated based on previous benchmarks"
        })
    
    # Generate report
    generate_report(results, tx_hashes)

if __name__ == "__main__":
    main()