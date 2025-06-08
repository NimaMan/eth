#!/usr/bin/env python3
"""
Improved Block Analyzer for Mempool Coverage Analysis

This version uses the tracking metadata to analyze the correct block range
that was active during mempool tracking.
"""

import json
import pandas as pd
from web3 import Web3
from datetime import datetime
from collections import defaultdict
import logging
from typing import Dict, List, Tuple
import os
import sys

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class ImprovedBlockAnalyzer:
    def __init__(self, w3_provider: str = "http://127.0.0.1:8545"):
        """Initialize with Web3 connection."""
        self.w3 = Web3(Web3.HTTPProvider(w3_provider))
        if not self.w3.is_connected():
            raise ConnectionError("Failed to connect to Ethereum node")
        
        self.mempool_transactions = {}
        self.mined_transactions = []
        self.coverage_stats = {}
        self.tracking_metadata = None
    
    def load_tracking_data(self, tx_file: str, metadata_file: str) -> None:
        """Load mempool transaction data and tracking metadata."""
        # Load metadata
        logger.info(f"📂 Loading tracking metadata from {metadata_file}")
        with open(metadata_file, 'r') as f:
            self.tracking_metadata = json.load(f)
        
        logger.info(f"📍 Tracking period: Block {self.tracking_metadata['start_block']} → {self.tracking_metadata['end_block']}")
        logger.info(f"⏱️  Duration: {self.tracking_metadata['tracking_duration_seconds']}s")
        
        # Log initial mempool info if available
        if 'initial_mempool_size' in self.tracking_metadata:
            logger.info(f"📦 Initial mempool size: {self.tracking_metadata['initial_mempool_size']}")
            logger.info(f"🆕 New transactions during tracking: {self.tracking_metadata.get('new_transactions', 'N/A')}")
        
        # Load mempool transactions
        logger.info(f"📂 Loading mempool data from {tx_file}")
        df = pd.read_csv(tx_file)
        
        # Process each transaction
        for _, row in df.iterrows():
            tx_hash = row['tx_hash'].lower()
            if tx_hash.startswith('0x'):
                tx_hash = tx_hash
            else:
                tx_hash = '0x' + tx_hash
            
            self.mempool_transactions[tx_hash] = {
                'mempool_arrival_time': row['mempool_arrival_time'],
                'processing_time_ms': row['processing_time_ms'],
                'gas_price': row.get('gas_price'),
                'value': row.get('value', '0'),
                'from_address': row.get('from_address', '').lower(),
                'to_address': row.get('to_address', '').lower() if pd.notna(row.get('to_address')) else None
            }
        
        logger.info(f"✅ Loaded {len(self.mempool_transactions)} mempool transactions")
    
    def analyze_blocks(self, additional_blocks: int = 10) -> None:
        """Analyze blocks from the tracking period plus some additional blocks."""
        if not self.tracking_metadata:
            raise ValueError("No tracking metadata loaded")
        
        start_block = self.tracking_metadata['start_block']
        end_block = self.tracking_metadata['end_block'] + additional_blocks  # Add buffer for delayed mining
        
        logger.info(f"🔍 Analyzing blocks {start_block} to {end_block} ({end_block - start_block + 1} blocks)")
        logger.info(f"   (includes {additional_blocks} additional blocks for delayed mining)")
        
        total_transactions = 0
        seen_in_mempool = 0
        mempool_to_mining_times = []
        
        # Analyze each block
        for block_num in range(start_block, end_block + 1):
            if (block_num - start_block) % 10 == 0:
                logger.info(f"   Processing block {block_num} ({block_num - start_block + 1}/{end_block - start_block + 1})")
            
            try:
                # Get block with transactions
                block = self.w3.eth.get_block(block_num, full_transactions=True)
                block_timestamp = block.timestamp
                
                # Process each transaction in the block
                for tx in block.transactions:
                    total_transactions += 1
                    tx_hash = tx.hash.hex().lower()
                    # Ensure hash has 0x prefix for comparison
                    if not tx_hash.startswith('0x'):
                        tx_hash = '0x' + tx_hash
                    
                    # Create mined transaction record
                    mined_tx = {
                        'tx_hash': tx_hash,
                        'block_number': block_num,
                        'block_timestamp': block_timestamp,
                        'gas_price': tx.gasPrice,
                        'gas_used': getattr(tx, 'gas', 0),
                        'value': tx.value,
                        'from_address': tx['from'].lower(),
                        'to_address': tx.to.lower() if tx.to else None,
                        'nonce': tx.nonce,
                        'was_in_mempool': False,
                        'mempool_to_mining_time': None,
                        'in_tracking_period': block_num <= self.tracking_metadata['end_block']
                    }
                    
                    # Check if this transaction was seen in mempool
                    if tx_hash in self.mempool_transactions:
                        seen_in_mempool += 1
                        mined_tx['was_in_mempool'] = True
                        
                        # Calculate mempool to mining time
                        mempool_time = self.mempool_transactions[tx_hash]['mempool_arrival_time']
                        mining_time = block_timestamp
                        time_diff = mining_time - mempool_time
                        
                        mined_tx['mempool_to_mining_time'] = time_diff
                        mempool_to_mining_times.append(time_diff)
                    
                    self.mined_transactions.append(mined_tx)
                
            except Exception as e:
                logger.warning(f"⚠️  Error processing block {block_num}: {e}")
                continue
        
        # Calculate statistics
        tracking_period_txs = sum(1 for tx in self.mined_transactions if tx['in_tracking_period'])
        
        self.coverage_stats.update({
            'tracking_start_block': start_block,
            'tracking_end_block': self.tracking_metadata['end_block'],
            'analyzed_end_block': end_block,
            'total_blocks_tracked': self.tracking_metadata['end_block'] - start_block + 1,
            'total_blocks_analyzed': end_block - start_block + 1,
            'total_mempool_tracked': len(self.mempool_transactions),
            'total_mined': total_transactions,
            'total_mined_in_tracking_period': tracking_period_txs,
            'seen_in_mempool': seen_in_mempool,
            'direct_to_block': total_transactions - seen_in_mempool,
            'mempool_coverage_rate': (seen_in_mempool / total_transactions * 100) if total_transactions > 0 else 0,
            'tracking_period_coverage_rate': (seen_in_mempool / tracking_period_txs * 100) if tracking_period_txs > 0 else 0,
            'avg_mempool_to_mining_time': sum(mempool_to_mining_times) / len(mempool_to_mining_times) if mempool_to_mining_times else 0,
            'mempool_to_mining_times': mempool_to_mining_times
        })
        
        logger.info(f"✅ Analysis complete: {total_transactions} total transactions")
        logger.info(f"📊 Transactions in tracking period: {tracking_period_txs}")
        logger.info(f"📊 Mempool coverage: {seen_in_mempool}/{total_transactions} ({self.coverage_stats['mempool_coverage_rate']:.1f}%)")
        logger.info(f"📊 Tracking period coverage: {seen_in_mempool}/{tracking_period_txs} ({self.coverage_stats['tracking_period_coverage_rate']:.1f}%)")
    
    def generate_report(self, output_prefix: str) -> None:
        """Generate detailed report with improved statistics."""
        report = {
            'analysis_timestamp': datetime.now().isoformat(),
            'tracking_metadata': self.tracking_metadata,
            'coverage_stats': self.coverage_stats,
            'detailed_stats': self.generate_detailed_analysis()
        }
        
        # Save report as JSON
        json_file = f"{output_prefix}_improved_analysis.json"
        with open(json_file, 'w') as f:
            json.dump(report, f, indent=2)
        
        logger.info(f"📄 Report saved to {json_file}")
        
        # Generate human-readable summary
        self.print_summary()
    
    def generate_detailed_analysis(self) -> dict:
        """Generate detailed analysis with breakdowns."""
        # Transaction type analysis
        contract_creations = sum(1 for tx in self.mined_transactions if tx['to_address'] is None)
        zero_value_txs = sum(1 for tx in self.mined_transactions if tx['value'] == 0)
        high_gas_txs = sum(1 for tx in self.mined_transactions if tx['gas_price'] > 50_000_000_000)  # > 50 gwei
        
        # Block-by-block coverage
        block_coverage = defaultdict(lambda: {'total': 0, 'mempool': 0})
        for tx in self.mined_transactions:
            block_num = tx['block_number']
            block_coverage[block_num]['total'] += 1
            if tx['was_in_mempool']:
                block_coverage[block_num]['mempool'] += 1
        
        # Calculate per-block coverage rates
        coverage_by_block = []
        for block_num in sorted(block_coverage.keys()):
            data = block_coverage[block_num]
            coverage_rate = (data['mempool'] / data['total'] * 100) if data['total'] > 0 else 0
            coverage_by_block.append({
                'block_number': block_num,
                'total_txs': data['total'],
                'mempool_txs': data['mempool'],
                'coverage_rate': coverage_rate,
                'in_tracking_period': block_num <= self.tracking_metadata['end_block']
            })
        
        return {
            'transaction_types': {
                'contract_creations': contract_creations,
                'zero_value_transactions': zero_value_txs,
                'high_gas_transactions': high_gas_txs
            },
            'coverage_by_block': coverage_by_block,
            'mempool_timing_distribution': self.analyze_timing_distribution()
        }
    
    def analyze_timing_distribution(self) -> dict:
        """Analyze distribution of mempool to mining times."""
        times = self.coverage_stats.get('mempool_to_mining_times', [])
        if not times:
            return {}
        
        times_sorted = sorted(times)
        n = len(times_sorted)
        
        return {
            'min': times_sorted[0],
            'max': times_sorted[-1],
            'median': times_sorted[n//2],
            'p25': times_sorted[n//4] if n >= 4 else times_sorted[0],
            'p75': times_sorted[3*n//4] if n >= 4 else times_sorted[-1],
            'p90': times_sorted[int(0.9*n)] if n >= 10 else times_sorted[-1],
            'p99': times_sorted[int(0.99*n)] if n >= 100 else times_sorted[-1]
        }
    
    def print_summary(self) -> None:
        """Print executive summary."""
        print("\n" + "="*60)
        print("🎯 IMPROVED MEMPOOL COVERAGE ANALYSIS")
        print("="*60)
        print(f"\n📍 TRACKING PERIOD:")
        print(f"   • Blocks: {self.tracking_metadata['start_block']} → {self.tracking_metadata['end_block']}")
        print(f"   • Duration: {self.tracking_metadata['tracking_duration_seconds']}s")
        print(f"   • Mempool transactions tracked: {len(self.mempool_transactions)}")
        
        print(f"\n📊 COVERAGE RESULTS:")
        print(f"   • Total transactions analyzed: {self.coverage_stats['total_mined']}")
        print(f"   • Transactions in tracking period: {self.coverage_stats['total_mined_in_tracking_period']}")
        print(f"   • Seen in mempool: {self.coverage_stats['seen_in_mempool']}")
        print(f"   • Overall coverage: {self.coverage_stats['mempool_coverage_rate']:.1f}%")
        print(f"   • Tracking period coverage: {self.coverage_stats['tracking_period_coverage_rate']:.1f}%")
        
        timing_dist = self.analyze_timing_distribution()
        if timing_dist:
            print(f"\n⏱️  MEMPOOL TO MINING TIMES:")
            print(f"   • Median: {timing_dist['median']:.1f}s")
            print(f"   • P90: {timing_dist['p90']:.1f}s")
            print(f"   • Max: {timing_dist['max']:.1f}s")
        
        print("\n" + "="*60)


def main():
    """Main function to run improved analysis."""
    import argparse
    
    parser = argparse.ArgumentParser(description='Analyze mempool coverage with proper block alignment')
    parser.add_argument('tx_file', help='Mempool transactions CSV file')
    parser.add_argument('metadata_file', help='Tracking metadata JSON file')
    parser.add_argument('--additional-blocks', type=int, default=10,
                        help='Additional blocks to analyze after tracking period (default: 10)')
    parser.add_argument('--output-prefix', default='coverage',
                        help='Output file prefix (default: coverage)')
    
    args = parser.parse_args()
    
    # Run analysis
    analyzer = ImprovedBlockAnalyzer()
    analyzer.load_tracking_data(args.tx_file, args.metadata_file)
    analyzer.analyze_blocks(args.additional_blocks)
    analyzer.generate_report(args.output_prefix)


if __name__ == "__main__":
    main()