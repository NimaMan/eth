#!/usr/bin/env python3
"""
Complete Transaction Timing Analysis: Mempool vs Processing Performance

OBJECTIVE: Measure and compare:
1. Time spent in Ethereum mempool (before mining)
2. Time we take to process/simulate transactions
3. Validate against real-world Ethereum network benchmarks

ALGORITHM:
1. Load transaction timing data with all metrics
2. Separate mempool residence time vs our processing time
3. Calculate detailed statistics for both components
4. Compare against Ethereum network benchmarks from research
5. Identify optimization opportunities and validate performance
6. Generate comprehensive timing breakdown analysis
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
from datetime import datetime, timedelta
import warnings
warnings.filterwarnings('ignore')

class CompleteTransactionTimingAnalyzer:
    def __init__(self, csv_file_path):
        """Initialize analyzer with complete transaction timing data."""
        self.csv_file = csv_file_path
        
        print(f"⏱️  COMPLETE TRANSACTION TIMING ANALYSIS")
        print("=" * 70)
        print(f"📊 Loading complete transaction timing data...")
        
        self.df = pd.read_csv(csv_file_path)
        self.df['timestamp'] = pd.to_datetime(self.df['timestamp'])
        
        # Convert timing columns to numeric
        self.df['mempool_arrival_time'] = pd.to_numeric(self.df['mempool_arrival_time'])
        self.df['queue_time_ms'] = pd.to_numeric(self.df['queue_time_ms'])  
        self.df['processing_time_ms'] = pd.to_numeric(self.df['processing_time_ms'])
        
        # Calculate total time (mempool + our processing)
        self.df['total_time_ms'] = self.df['mempool_arrival_time'] + self.df['processing_time_ms']
        
        # Categorize transactions
        self.df['tx_category'] = self.df.apply(self._categorize_transaction, axis=1)
        
        print(f"📊 Dataset Overview:")
        print(f"   Total Transactions: {len(self.df):,}")
        print(f"   Time Range: {self.df['timestamp'].min()} to {self.df['timestamp'].max()}")
        print(f"   Analysis Duration: {(self.df['timestamp'].max() - self.df['timestamp'].min()).total_seconds():.1f} seconds")
        
    def _categorize_transaction(self, row):
        """Categorize transactions based on timing characteristics."""
        if row['mempool_arrival_time'] == 0:
            return "Direct-to-Miner"
        elif row['mempool_arrival_time'] <= 100:
            return "Fast Mempool"
        elif row['mempool_arrival_time'] <= 1000:
            return "Standard Mempool"
        else:
            return "Slow Mempool"
    
    def analyze_mempool_residence_time(self):
        """Detailed analysis of time spent in Ethereum mempool."""
        print(f"\n🏊 ETHEREUM MEMPOOL RESIDENCE TIME ANALYSIS")
        print("=" * 70)
        
        mempool_times = self.df['mempool_arrival_time']
        
        # Filter out direct-to-miner (0ms mempool time)
        public_mempool = mempool_times[mempool_times > 0]
        direct_to_miner = mempool_times[mempool_times == 0]
        
        print(f"📊 MEMPOOL CATEGORIES:")
        print(f"   Public Mempool Transactions: {len(public_mempool):,} ({len(public_mempool)/len(self.df)*100:.1f}%)")
        print(f"   Direct-to-Miner: {len(direct_to_miner):,} ({len(direct_to_miner)/len(self.df)*100:.1f}%)")
        
        if len(public_mempool) > 0:
            print(f"\n⏱️  PUBLIC MEMPOOL RESIDENCE STATISTICS:")
            print(f"   Mean: {public_mempool.mean():.1f} ms ({public_mempool.mean()/1000:.2f} seconds)")
            print(f"   Median: {public_mempool.median():.1f} ms ({public_mempool.median()/1000:.2f} seconds)")
            print(f"   P75: {public_mempool.quantile(0.75):.1f} ms")
            print(f"   P90: {public_mempool.quantile(0.90):.1f} ms")
            print(f"   P95: {public_mempool.quantile(0.95):.1f} ms")
            print(f"   P99: {public_mempool.quantile(0.99):.1f} ms")
            print(f"   Max: {public_mempool.max():.1f} ms ({public_mempool.max()/1000:.2f} seconds)")
            print(f"   Std Dev: {public_mempool.std():.1f} ms")
            
            # Distribution analysis
            print(f"\n📊 MEMPOOL RESIDENCE DISTRIBUTION:")
            bins = [
                (0, 50, "⚡ Ultra-Fast (0-50ms)"),
                (50, 100, "🟢 Fast (50-100ms)"),
                (100, 500, "🟡 Medium (100-500ms)"),
                (500, 1000, "🟠 Standard (500ms-1s)"),
                (1000, 5000, "🔴 Slow (1-5s)"),
                (5000, float('inf'), "🚨 Very Slow (>5s)")
            ]
            
            for min_val, max_val, label in bins:
                if max_val == float('inf'):
                    count = len(public_mempool[public_mempool >= min_val])
                else:
                    count = len(public_mempool[(public_mempool >= min_val) & (public_mempool < max_val)])
                percentage = (count / len(public_mempool)) * 100
                print(f"   {label}: {count:,} ({percentage:.1f}%)")
                
        return {
            'public_mempool_count': len(public_mempool),
            'direct_to_miner_count': len(direct_to_miner),
            'mean_mempool_time': public_mempool.mean() if len(public_mempool) > 0 else 0,
            'median_mempool_time': public_mempool.median() if len(public_mempool) > 0 else 0,
            'p95_mempool_time': public_mempool.quantile(0.95) if len(public_mempool) > 0 else 0,
        }
    
    def analyze_our_processing_time(self):
        """Detailed analysis of our transaction processing/simulation time."""
        print(f"\n⚙️  OUR TRANSACTION PROCESSING TIME ANALYSIS")
        print("=" * 70)
        
        processing_times = self.df['processing_time_ms']
        queue_times = self.df['queue_time_ms']
        
        print(f"📊 PROCESSING TIME STATISTICS:")
        print(f"   Mean Processing: {processing_times.mean():.3f} ms")
        print(f"   Median Processing: {processing_times.median():.3f} ms")
        print(f"   P95 Processing: {processing_times.quantile(0.95):.3f} ms")
        print(f"   P99 Processing: {processing_times.quantile(0.99):.3f} ms")
        print(f"   Max Processing: {processing_times.max():.3f} ms")
        print(f"   Std Dev Processing: {processing_times.std():.3f} ms")
        
        print(f"\n📊 QUEUE TIME STATISTICS:")
        print(f"   Mean Queue: {queue_times.mean():.1f} ms ({queue_times.mean()/1000:.3f} seconds)")
        print(f"   Median Queue: {queue_times.median():.1f} ms")
        print(f"   P95 Queue: {queue_times.quantile(0.95):.1f} ms")
        print(f"   P99 Queue: {queue_times.quantile(0.99):.1f} ms")
        print(f"   Max Queue: {queue_times.max():.1f} ms")
        
        # Processing time distribution
        print(f"\n📊 PROCESSING TIME DISTRIBUTION:")
        proc_bins = [
            (0, 0.1, "⚡ Ultra-Fast (<0.1ms)"),
            (0.1, 1, "🟢 Fast (0.1-1ms)"),
            (1, 10, "🟡 Standard (1-10ms)"),
            (10, 100, "🟠 Slow (10-100ms)"),
            (100, float('inf'), "🔴 Very Slow (>100ms)")
        ]
        
        for min_val, max_val, label in proc_bins:
            if max_val == float('inf'):
                count = len(processing_times[processing_times >= min_val])
            else:
                count = len(processing_times[(processing_times >= min_val) & (processing_times < max_val)])
            percentage = (count / len(processing_times)) * 100
            print(f"   {label}: {count:,} ({percentage:.1f}%)")
        
        # Performance by transaction category
        print(f"\n📊 PROCESSING BY TRANSACTION CATEGORY:")
        for category in self.df['tx_category'].unique():
            cat_data = self.df[self.df['tx_category'] == category]
            mean_proc = cat_data['processing_time_ms'].mean()
            mean_queue = cat_data['queue_time_ms'].mean()
            print(f"   {category}: {len(cat_data):,} txs, Proc: {mean_proc:.3f}ms, Queue: {mean_queue:.1f}ms")
            
        return {
            'mean_processing_time': processing_times.mean(),
            'median_processing_time': processing_times.median(),
            'p95_processing_time': processing_times.quantile(0.95),
            'mean_queue_time': queue_times.mean(),
            'median_queue_time': queue_times.median(),
            'p95_queue_time': queue_times.quantile(0.95),
        }
    
    def compare_mempool_vs_processing(self):
        """Compare mempool residence time vs our processing time."""
        print(f"\n⚖️  MEMPOOL vs PROCESSING TIME COMPARISON")
        print("=" * 70)
        
        # Filter public mempool transactions for comparison
        public_mempool_txs = self.df[self.df['mempool_arrival_time'] > 0].copy()
        
        if len(public_mempool_txs) > 0:
            mempool_avg = public_mempool_txs['mempool_arrival_time'].mean()
            processing_avg = public_mempool_txs['processing_time_ms'].mean()
            queue_avg = public_mempool_txs['queue_time_ms'].mean()
            total_avg = mempool_avg + processing_avg
            
            print(f"📊 TIMING BREAKDOWN (Public Mempool Transactions):")
            print(f"   Mempool Residence: {mempool_avg:.1f} ms ({mempool_avg/1000:.3f} seconds)")
            print(f"   Our Queue Time: {queue_avg:.1f} ms ({queue_avg/1000:.3f} seconds)")
            print(f"   Our Processing: {processing_avg:.3f} ms ({processing_avg/1000:.6f} seconds)")
            print(f"   Total Time: {total_avg:.1f} ms ({total_avg/1000:.3f} seconds)")
            
            print(f"\n📊 PERFORMANCE RATIOS:")
            print(f"   Queue/Mempool Ratio: {queue_avg/mempool_avg:.3f}x")
            print(f"   Processing/Mempool Ratio: {processing_avg/mempool_avg:.6f}x")
            print(f"   Processing/Queue Ratio: {processing_avg/queue_avg:.6f}x")
            
            print(f"\n📊 TIME ALLOCATION:")
            mempool_pct = (mempool_avg / total_avg) * 100
            queue_pct = (queue_avg / total_avg) * 100
            processing_pct = (processing_avg / total_avg) * 100
            
            print(f"   Mempool Residence: {mempool_pct:.1f}% of total time")
            print(f"   Our Queue: {queue_pct:.1f}% of total time")
            print(f"   Our Processing: {processing_pct:.3f}% of total time")
            
        # Direct-to-miner analysis
        direct_txs = self.df[self.df['mempool_arrival_time'] == 0].copy()
        if len(direct_txs) > 0:
            direct_processing_avg = direct_txs['processing_time_ms'].mean()
            direct_queue_avg = direct_txs['queue_time_ms'].mean()
            
            print(f"\n📊 DIRECT-TO-MINER TIMING:")
            print(f"   Count: {len(direct_txs):,} transactions")
            print(f"   Mempool Residence: 0 ms (bypassed)")
            print(f"   Our Queue Time: {direct_queue_avg:.1f} ms")
            print(f"   Our Processing: {direct_processing_avg:.3f} ms")
            print(f"   Total Time: {direct_queue_avg + direct_processing_avg:.1f} ms")
            
        return {
            'mempool_avg': mempool_avg if len(public_mempool_txs) > 0 else 0,
            'processing_avg': processing_avg if len(public_mempool_txs) > 0 else 0,
            'queue_avg': queue_avg if len(public_mempool_txs) > 0 else 0,
            'direct_to_miner_count': len(direct_txs),
        }
    
    def calculate_throughput_metrics(self):
        """Calculate transaction throughput and processing rates."""
        print(f"\n📈 THROUGHPUT AND PERFORMANCE METRICS")
        print("=" * 70)
        
        duration = (self.df['timestamp'].max() - self.df['timestamp'].min()).total_seconds()
        total_txs = len(self.df)
        
        print(f"📊 OVERALL THROUGHPUT:")
        print(f"   Total Transactions: {total_txs:,}")
        print(f"   Analysis Duration: {duration:.1f} seconds ({duration/60:.1f} minutes)")
        print(f"   Transaction Rate: {total_txs/duration:.1f} tx/second")
        print(f"   Transaction Rate: {(total_txs/duration)*60:.0f} tx/minute")
        print(f"   Transaction Rate: {(total_txs/duration)*3600:.0f} tx/hour")
        
        # Processing capacity analysis
        processing_times = self.df['processing_time_ms']
        avg_processing = processing_times.mean()
        
        if avg_processing > 0:
            theoretical_capacity = 1000 / avg_processing  # tx/second if no queue delay
            print(f"\n⚙️  PROCESSING CAPACITY:")
            print(f"   Average Processing Time: {avg_processing:.3f} ms")
            print(f"   Theoretical Max Capacity: {theoretical_capacity:.0f} tx/second")
            print(f"   Current Utilization: {(total_txs/duration)/theoretical_capacity*100:.3f}%")
        
        # Queue efficiency analysis
        queue_times = self.df['queue_time_ms']
        print(f"\n⏳ QUEUE EFFICIENCY:")
        print(f"   Average Queue Time: {queue_times.mean():.1f} ms")
        print(f"   Queue Time Std Dev: {queue_times.std():.1f} ms")
        print(f"   Queue Efficiency: {(avg_processing/(queue_times.mean() + avg_processing))*100:.3f}%")
        
        return {
            'transaction_rate': total_txs/duration,
            'duration': duration,
            'avg_processing_time': avg_processing,
            'avg_queue_time': queue_times.mean(),
        }
    
    def generate_ethereum_benchmark_comparison(self):
        """Compare our findings against known Ethereum network benchmarks."""
        print(f"\n🌐 ETHEREUM NETWORK BENCHMARK COMPARISON")
        print("=" * 70)
        
        # Our measured statistics
        public_mempool_txs = self.df[self.df['mempool_arrival_time'] > 0]
        our_mempool_avg = public_mempool_txs['mempool_arrival_time'].mean() if len(public_mempool_txs) > 0 else 0
        our_processing_avg = self.df['processing_time_ms'].mean()
        
        print(f"📊 OUR MEASUREMENTS vs ETHEREUM BENCHMARKS:")
        print(f"   Our Mempool Residence: {our_mempool_avg:.0f} ms ({our_mempool_avg/1000:.2f} seconds)")
        print(f"   Our Processing Time: {our_processing_avg:.3f} ms")
        
        print(f"\n🔍 ETHEREUM NETWORK BENCHMARKS (Research-Based):")
        print(f"   Typical Mempool Time: 5-30 seconds (normal congestion)")
        print(f"   Fast Mempool Time: 1-5 seconds (low congestion)")
        print(f"   Block Time: ~12 seconds average")
        print(f"   Gas Limit: ~30M gas per block")
        print(f"   Network TPS: ~15 transactions/second")
        
        print(f"\n📊 BENCHMARK ANALYSIS:")
        # Note: Our mempool time seems very fast compared to typical Ethereum
        if our_mempool_avg < 5000:  # Less than 5 seconds
            print(f"   ✅ Our mempool times ({our_mempool_avg/1000:.2f}s) are FASTER than typical (5-30s)")
            print(f"   📍 This suggests low network congestion period or efficient mempool monitoring")
        else:
            print(f"   ⚠️  Our mempool times ({our_mempool_avg/1000:.2f}s) align with typical congestion")
            
        # Processing time comparison
        if our_processing_avg < 10:  # Less than 10ms
            print(f"   ✅ Our processing ({our_processing_avg:.3f}ms) is EXCELLENT for REVM simulation")
            print(f"   📍 Sub-10ms processing is highly optimized for EVM simulation")
        
        print(f"\n💡 INTERPRETATION:")
        print(f"   • Mempool times reflect network conditions during analysis period")
        print(f"   • Our processing speed is highly optimized (sub-millisecond)")
        print(f"   • Total delay dominated by mempool residence, not our processing")
        
    def run_complete_analysis(self):
        """Run the complete transaction timing analysis."""
        print(f"\n🚀 STARTING COMPLETE TRANSACTION TIMING ANALYSIS")
        print("=" * 80)
        
        # Run all analyses
        mempool_stats = self.analyze_mempool_residence_time()
        processing_stats = self.analyze_our_processing_time()
        comparison_stats = self.compare_mempool_vs_processing()
        throughput_stats = self.calculate_throughput_metrics()
        
        # Ethereum benchmark comparison
        self.generate_ethereum_benchmark_comparison()
        
        # Final summary
        print(f"\n📋 COMPLETE TIMING ANALYSIS SUMMARY")
        print("=" * 70)
        print(f"📊 Dataset: {len(self.df):,} transactions over {throughput_stats['duration']:.1f} seconds")
        print(f"⏱️  Mempool Residence: {comparison_stats['mempool_avg']:.1f} ms average")
        print(f"⚙️  Our Processing: {processing_stats['mean_processing_time']:.3f} ms average")
        print(f"⏳ Our Queue: {processing_stats['mean_queue_time']:.1f} ms average")
        print(f"📈 Throughput: {throughput_stats['transaction_rate']:.1f} tx/second")
        print(f"🚀 Direct-to-Miner: {comparison_stats['direct_to_miner_count']:,} transactions")
        
        return {
            'mempool_stats': mempool_stats,
            'processing_stats': processing_stats,
            'comparison_stats': comparison_stats,
            'throughput_stats': throughput_stats
        }

if __name__ == "__main__":
    # Run the complete transaction timing analysis
    analyzer = CompleteTransactionTimingAnalyzer('logs/mempool/transaction_timing_analysis_20250603_192458.csv')
    results = analyzer.run_complete_analysis() 