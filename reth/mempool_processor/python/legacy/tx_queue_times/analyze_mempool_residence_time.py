#!/usr/bin/env python3
"""
Mempool Residence Time and Direct-to-Miner Transaction Analysis

OBJECTIVE: Analyze how time was spent in the mempool before transactions were mined,
including identification of transactions that bypass the public mempool entirely
(direct-to-miner with zero mempool time).

ALGORITHM:
1. Load transaction data and parse mempool_arrival_time (actual mempool residence)
2. Identify zero-mempool-time transactions (direct-to-miner)
3. Analyze mempool residence time distributions
4. Compare public mempool vs direct-to-miner transaction patterns
5. Document complete transaction lifecycle including private channels
6. Generate comprehensive mempool residence analysis
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
from datetime import datetime, timedelta
import warnings
warnings.filterwarnings('ignore')

class MempoolResidenceAnalyzer:
    def __init__(self, csv_file_path):
        """Initialize analyzer with transaction timing data."""
        self.csv_file = csv_file_path
        
        print(f"🏊 MEMPOOL RESIDENCE TIME ANALYSIS")
        print("=" * 60)
        print(f"📊 Loading transaction data...")
        
        self.df = pd.read_csv(csv_file_path)
        self.df['timestamp'] = pd.to_datetime(self.df['timestamp'])
        
        # Convert timing columns
        self.df['mempool_arrival_time'] = pd.to_numeric(self.df['mempool_arrival_time'])
        self.df['queue_time_ms'] = pd.to_numeric(self.df['queue_time_ms'])
        self.df['processing_time_ms'] = pd.to_numeric(self.df['processing_time_ms'])
        
        # Categorize transactions by mempool behavior
        self.df['tx_type'] = self.df['mempool_arrival_time'].apply(self._categorize_transaction)
        
        # Separate transaction types
        self.direct_to_miner = self.df[self.df['mempool_arrival_time'] == 0].copy()
        self.public_mempool = self.df[self.df['mempool_arrival_time'] > 0].copy()
        
        print(f"📊 Total Transactions: {len(self.df):,}")
        print(f"🚀 Direct-to-Miner (0ms mempool): {len(self.direct_to_miner):,}")
        print(f"🏊 Public Mempool (>0ms): {len(self.public_mempool):,}")
        print(f"📅 Time Range: {self.df['timestamp'].min()} to {self.df['timestamp'].max()}")
        print(f"⏱️  Total Duration: {(self.df['timestamp'].max() - self.df['timestamp'].min()).total_seconds():.1f} seconds")
        
    def _categorize_transaction(self, mempool_time):
        """Categorize transactions based on mempool residence time."""
        if mempool_time == 0:
            return "Direct-to-Miner"
        elif mempool_time <= 10:
            return "Ultra-Fast (≤10ms)"
        elif mempool_time <= 100:
            return "Fast (≤100ms)"
        elif mempool_time <= 1000:
            return "Standard (≤1s)"
        elif mempool_time <= 10000:
            return "Slow (≤10s)"
        else:
            return "Very Slow (>10s)"
    
    def analyze_direct_to_miner_transactions(self):
        """Analyze transactions that bypassed the public mempool."""
        print(f"\n🚀 DIRECT-TO-MINER TRANSACTION ANALYSIS")
        print("=" * 60)
        
        if len(self.direct_to_miner) == 0:
            print("⚠️  No direct-to-miner transactions found")
            return
        
        print(f"📊 DIRECT-TO-MINER STATISTICS:")
        print(f"   Count: {len(self.direct_to_miner):,} transactions")
        print(f"   Percentage of Total: {len(self.direct_to_miner)/len(self.df)*100:.2f}%")
        
        # Analyze their queue time in our system
        queue_times = self.direct_to_miner['queue_time_ms']
        print(f"\n⏱️  OUR SYSTEM QUEUE TIMES (for direct-to-miner txs):")
        print(f"   Mean Queue Time: {queue_times.mean():.1f} ms")
        print(f"   Median Queue Time: {queue_times.median():.1f} ms")
        print(f"   P95 Queue Time: {queue_times.quantile(0.95):.1f} ms")
        print(f"   Max Queue Time: {queue_times.max():.1f} ms")
        
        # Time distribution
        duration = (self.direct_to_miner['timestamp'].max() - self.direct_to_miner['timestamp'].min()).total_seconds()
        rate = len(self.direct_to_miner) / duration if duration > 0 else 0
        
        print(f"\n📈 DIRECT-TO-MINER THROUGHPUT:")
        print(f"   Rate: {rate:.2f} direct-to-miner tx/second")
        print(f"   Rate: {rate * 60:.1f} direct-to-miner tx/minute")
        
        print(f"\n🔍 DIRECT-TO-MINER INSIGHTS:")
        print(f"   • These transactions likely used private mempools (Flashbots, etc.)")
        print(f"   • They appeared in blocks without public mempool visibility")
        print(f"   • Represent {len(self.direct_to_miner)/len(self.df)*100:.1f}% of all transactions")
        print(f"   • Still processed through our queue with {queue_times.mean():.0f}ms average delay")
        
        return {
            'count': len(self.direct_to_miner),
            'percentage': len(self.direct_to_miner)/len(self.df)*100,
            'mean_queue_time': queue_times.mean(),
            'median_queue_time': queue_times.median(),
            'rate': rate
        }
    
    def analyze_public_mempool_transactions(self):
        """Analyze transactions that went through the public mempool."""
        print(f"\n🏊 PUBLIC MEMPOOL TRANSACTION ANALYSIS")
        print("=" * 60)
        
        if len(self.public_mempool) == 0:
            print("⚠️  No public mempool transactions found")
            return
        
        mempool_times = self.public_mempool['mempool_arrival_time']
        queue_times = self.public_mempool['queue_time_ms']
        
        print(f"📊 PUBLIC MEMPOOL STATISTICS:")
        print(f"   Count: {len(self.public_mempool):,} transactions")
        print(f"   Percentage of Total: {len(self.public_mempool)/len(self.df)*100:.2f}%")
        
        print(f"\n⏱️  MEMPOOL RESIDENCE TIME:")
        print(f"   Mean Mempool Time: {mempool_times.mean():.1f} ms")
        print(f"   Median Mempool Time: {mempool_times.median():.1f} ms")
        print(f"   P75 Mempool Time: {mempool_times.quantile(0.75):.1f} ms")
        print(f"   P90 Mempool Time: {mempool_times.quantile(0.90):.1f} ms")
        print(f"   P95 Mempool Time: {mempool_times.quantile(0.95):.1f} ms")
        print(f"   P99 Mempool Time: {mempool_times.quantile(0.99):.1f} ms")
        print(f"   Max Mempool Time: {mempool_times.max():.1f} ms")
        
        print(f"\n⏱️  OUR SYSTEM QUEUE TIMES (for public mempool txs):")
        print(f"   Mean Queue Time: {queue_times.mean():.1f} ms")
        print(f"   Median Queue Time: {queue_times.median():.1f} ms")
        print(f"   P95 Queue Time: {queue_times.quantile(0.95):.1f} ms")
        print(f"   Max Queue Time: {queue_times.max():.1f} ms")
        
        # Conversion rates
        mempool_seconds = mempool_times.mean() / 1000
        queue_seconds = queue_times.mean() / 1000
        
        print(f"\n📊 TIMING COMPARISON:")
        print(f"   Average Mempool Residence: {mempool_seconds:.2f} seconds")
        print(f"   Average Our Queue Time: {queue_seconds:.2f} seconds")
        print(f"   Our Queue vs Mempool Ratio: {queue_seconds/mempool_seconds:.1f}x")
        
        return {
            'count': len(self.public_mempool),
            'percentage': len(self.public_mempool)/len(self.df)*100,
            'mean_mempool_time': mempool_times.mean(),
            'median_mempool_time': mempool_times.median(),
            'p95_mempool_time': mempool_times.quantile(0.95),
            'max_mempool_time': mempool_times.max(),
            'mean_queue_time': queue_times.mean(),
            'queue_to_mempool_ratio': queue_seconds/mempool_seconds if mempool_seconds > 0 else 0
        }
    
    def analyze_mempool_residence_distribution(self):
        """Analyze the distribution of mempool residence times."""
        print(f"\n📊 MEMPOOL RESIDENCE TIME DISTRIBUTION")
        print("=" * 60)
        
        # Create detailed distribution analysis
        total_transactions = len(self.df)
        
        print(f"COMPLETE TRANSACTION LIFECYCLE ANALYSIS:")
        
        # Transaction type distribution
        type_counts = self.df['tx_type'].value_counts()
        for tx_type, count in type_counts.items():
            percentage = (count / total_transactions) * 100
            print(f"   {tx_type}: {count:,} transactions ({percentage:.1f}%)")
        
        # Detailed time buckets for public mempool transactions
        if len(self.public_mempool) > 0:
            print(f"\nPUBLIC MEMPOOL RESIDENCE TIME BUCKETS:")
            mempool_times = self.public_mempool['mempool_arrival_time']
            
            buckets = [
                (0, 10, "⚡ 0-10ms (Instant)"),
                (10, 100, "🟢 10-100ms (Very Fast)"),
                (100, 1000, "🟡 100ms-1s (Fast)"),
                (1000, 10000, "🟠 1-10s (Standard)"),
                (10000, 60000, "🔴 10-60s (Slow)"),
                (60000, float('inf'), "🚨 >1min (Very Slow)")
            ]
            
            for min_time, max_time, label in buckets:
                if max_time == float('inf'):
                    count = len(mempool_times[mempool_times >= min_time])
                else:
                    count = len(mempool_times[(mempool_times >= min_time) & (mempool_times < max_time)])
                
                if len(mempool_times) > 0:
                    percentage = (count / len(mempool_times)) * 100
                    print(f"   {label}: {count:,} transactions ({percentage:.1f}%)")
    
    def analyze_transaction_lifecycle(self):
        """Analyze the complete transaction lifecycle from mempool to mining."""
        print(f"\n🔄 COMPLETE TRANSACTION LIFECYCLE ANALYSIS")
        print("=" * 60)
        
        print(f"TRANSACTION FLOW PATTERNS:")
        
        # Direct-to-miner flow
        direct_count = len(self.direct_to_miner)
        direct_pct = (direct_count / len(self.df)) * 100
        print(f"\n🚀 DIRECT-TO-MINER FLOW ({direct_pct:.1f}% of transactions):")
        print(f"   Private Mempool → Miner → Block → Our Detection")
        print(f"   Mempool Residence: 0ms (bypassed public mempool)")
        print(f"   Our Detection Delay: {self.direct_to_miner['queue_time_ms'].mean():.0f}ms average")
        print(f"   Likely Channels: Flashbots, private pools, direct miner relationships")
        
        # Public mempool flow
        public_count = len(self.public_mempool)
        public_pct = (public_count / len(self.df)) * 100
        if public_count > 0:
            print(f"\n🏊 PUBLIC MEMPOOL FLOW ({public_pct:.1f}% of transactions):")
            print(f"   Public Mempool → Miner Selection → Block → Our Detection")
            print(f"   Mempool Residence: {self.public_mempool['mempool_arrival_time'].mean():.0f}ms average")
            print(f"   Our Detection Delay: {self.public_mempool['queue_time_ms'].mean():.0f}ms average")
            print(f"   Total Time: {(self.public_mempool['mempool_arrival_time'].mean() + self.public_mempool['queue_time_ms'].mean()):.0f}ms from mempool to our processing")
        
        print(f"\n📈 TIMING IMPLICATIONS:")
        if public_count > 0:
            mempool_avg = self.public_mempool['mempool_arrival_time'].mean()
            queue_avg = self.public_mempool['queue_time_ms'].mean()
            total_delay = mempool_avg + queue_avg
            
            print(f"   Public Mempool Transactions:")
            print(f"     - Time in mempool: {mempool_avg:.0f}ms")
            print(f"     - Time in our queue: {queue_avg:.0f}ms")
            print(f"     - Total delay: {total_delay:.0f}ms")
            print(f"     - Our queue is {queue_avg/mempool_avg:.1f}x mempool residence time")
        
        if direct_count > 0:
            direct_queue_avg = self.direct_to_miner['queue_time_ms'].mean()
            print(f"   Direct-to-Miner Transactions:")
            print(f"     - Time in mempool: 0ms (bypassed)")
            print(f"     - Time in our queue: {direct_queue_avg:.0f}ms")
            print(f"     - Total delay: {direct_queue_avg:.0f}ms")
    
    def generate_recommendations(self, direct_stats, public_stats):
        """Generate recommendations based on mempool residence analysis."""
        print(f"\n💡 MEMPOOL RESIDENCE OPTIMIZATION RECOMMENDATIONS")
        print("=" * 60)
        
        direct_pct = direct_stats['percentage'] if direct_stats else 0
        
        print(f"🎯 TRANSACTION TYPE INSIGHTS:")
        print(f"   Direct-to-Miner: {direct_pct:.1f}% of transactions")
        print(f"   Public Mempool: {100-direct_pct:.1f}% of transactions")
        
        if direct_pct > 10:
            print(f"\n🚀 HIGH DIRECT-TO-MINER ACTIVITY ({direct_pct:.1f}%):")
            print(f"   • Significant private mempool usage (Flashbots, MEV)")
            print(f"   • Consider monitoring private mempool channels")
            print(f"   • Implement direct miner relationship detection")
            print(f"   • Track transaction origin patterns")
        
        if public_stats and public_stats['queue_to_mempool_ratio'] > 2:
            print(f"\n⚠️  QUEUE DELAY vs MEMPOOL RESIDENCE:")
            print(f"   • Our queue time ({public_stats['mean_queue_time']:.0f}ms) > mempool residence ({public_stats['mean_mempool_time']:.0f}ms)")
            print(f"   • Ratio: {public_stats['queue_to_mempool_ratio']:.1f}x mempool time")
            print(f"   • Priority: Optimize our detection speed")
            print(f"   • Target: Match or beat mempool residence time")
        
        print(f"\n🔧 OPTIMIZATION PRIORITIES:")
        print(f"   1. **Real-time Block Monitoring**: Subscribe to block events directly")
        print(f"   2. **Private Mempool Monitoring**: Track Flashbots and private pools")
        print(f"   3. **Reduce Detection Latency**: Our queue time is significant vs mempool time")
        print(f"   4. **Transaction Origin Tracking**: Identify direct-to-miner patterns")
        print(f"   5. **Multi-source Monitoring**: Combine public/private mempool data")
    
    def run_complete_analysis(self):
        """Run the complete mempool residence time analysis."""
        print(f"\n🚀 STARTING COMPLETE MEMPOOL RESIDENCE ANALYSIS")
        print("=" * 70)
        
        # Run individual analyses
        direct_stats = self.analyze_direct_to_miner_transactions()
        public_stats = self.analyze_public_mempool_transactions()
        
        # Distribution analysis
        self.analyze_mempool_residence_distribution()
        
        # Transaction lifecycle
        self.analyze_transaction_lifecycle()
        
        # Recommendations
        self.generate_recommendations(direct_stats, public_stats)
        
        # Summary
        print(f"\n📋 MEMPOOL RESIDENCE SUMMARY")
        print("=" * 60)
        print(f"📊 Total Transactions: {len(self.df):,}")
        
        if direct_stats:
            print(f"🚀 Direct-to-Miner: {direct_stats['count']:,} ({direct_stats['percentage']:.1f}%)")
            print(f"   Average Queue Time: {direct_stats['mean_queue_time']:.0f}ms")
        
        if public_stats:
            print(f"🏊 Public Mempool: {public_stats['count']:,} ({public_stats['percentage']:.1f}%)")
            print(f"   Average Mempool Time: {public_stats['mean_mempool_time']:.0f}ms")
            print(f"   Average Queue Time: {public_stats['mean_queue_time']:.0f}ms")
            print(f"   Queue/Mempool Ratio: {public_stats['queue_to_mempool_ratio']:.1f}x")
        
        return direct_stats, public_stats

if __name__ == "__main__":
    # Run the mempool residence analysis
    analyzer = MempoolResidenceAnalyzer('logs/mempool/transaction_timing_analysis_20250603_192458.csv')
    direct_stats, public_stats = analyzer.run_complete_analysis() 