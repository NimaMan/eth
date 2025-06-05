#!/usr/bin/env python3
"""
Warmup vs Post-Warmup Queue Performance Analysis

OBJECTIVE: Analyze transaction queue performance before and after the 75,000 transaction warmup period
to identify timing differences, performance improvements, and system behavior changes.

ALGORITHM:
1. Load complete transaction dataset (222,630+ transactions)
2. Identify warmup period (first 75,000 transactions)
3. Separate pre-warmup vs post-warmup data
4. Calculate detailed queue timing statistics for both periods
5. Compare performance metrics and identify improvements
6. Generate comprehensive timing analysis report
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
from datetime import datetime, timedelta
import warnings
warnings.filterwarnings('ignore')

class WarmupQueueAnalyzer:
    def __init__(self, csv_file_path, warmup_target=150000):
        """Initialize analyzer with warmup threshold."""
        self.warmup_target = warmup_target
        self.csv_file = csv_file_path
        
        print(f"🔍 WARMUP VS POST-WARMUP QUEUE ANALYSIS")
        print("=" * 60)
        print(f"📊 Loading transaction data...")
        
        self.df = pd.read_csv(csv_file_path)
        self.df['timestamp'] = pd.to_datetime(self.df['timestamp'])
        
        # Convert timing columns
        self.df['queue_time_ms'] = pd.to_numeric(self.df['queue_time_ms'])
        self.df['processing_time_ms'] = pd.to_numeric(self.df['processing_time_ms'])
        
        # Sort by timestamp to establish order
        self.df = self.df.sort_values('timestamp').reset_index(drop=True)
        
        # Add transaction number for warmup analysis
        self.df['tx_number'] = range(1, len(self.df) + 1)
        
        # Separate warmup vs post-warmup periods
        self.warmup_df = self.df[self.df['tx_number'] <= warmup_target].copy()
        self.post_warmup_df = self.df[self.df['tx_number'] > warmup_target].copy()
        
        print(f"📊 Total Transactions: {len(self.df):,}")
        print(f"🔥 Warmup Period: {len(self.warmup_df):,} transactions")
        print(f"⚡ Post-Warmup Period: {len(self.post_warmup_df):,} transactions")
        print(f"📅 Time Range: {self.df['timestamp'].min()} to {self.df['timestamp'].max()}")
        print(f"⏱️  Total Duration: {(self.df['timestamp'].max() - self.df['timestamp'].min()).total_seconds():.1f} seconds")
        
    def analyze_warmup_period(self):
        """Detailed analysis of warmup period performance."""
        print(f"\n🔥 WARMUP PERIOD ANALYSIS (First {self.warmup_target:,} transactions)")
        print("=" * 60)
        
        if len(self.warmup_df) == 0:
            print("⚠️  No warmup data available")
            return
        
        queue_times = self.warmup_df['queue_time_ms']
        processing_times = self.warmup_df['processing_time_ms']
        
        # Timing statistics
        print(f"⏱️  QUEUE TIMING STATISTICS:")
        print(f"   Mean Queue Time: {queue_times.mean():.1f} ms")
        print(f"   Median Queue Time: {queue_times.median():.1f} ms")
        print(f"   P75 Queue Time: {queue_times.quantile(0.75):.1f} ms")
        print(f"   P90 Queue Time: {queue_times.quantile(0.90):.1f} ms") 
        print(f"   P95 Queue Time: {queue_times.quantile(0.95):.1f} ms")
        print(f"   P99 Queue Time: {queue_times.quantile(0.99):.1f} ms")
        print(f"   Max Queue Time: {queue_times.max():.1f} ms")
        print(f"   Std Dev: {queue_times.std():.1f} ms")
        
        print(f"\n⚙️  PROCESSING TIMING:")
        print(f"   Mean Processing Time: {processing_times.mean():.2f} ms")
        print(f"   Median Processing Time: {processing_times.median():.2f} ms")
        print(f"   Max Processing Time: {processing_times.max():.2f} ms")
        
        # SLA Analysis
        sla_violations = len(queue_times[queue_times > 100])
        sla_compliance = (1 - sla_violations / len(queue_times)) * 100
        
        print(f"\n🎯 SLA COMPLIANCE (100ms target):")
        print(f"   Violations: {sla_violations:,} transactions ({sla_violations/len(queue_times)*100:.1f}%)")
        print(f"   Compliance: {sla_compliance:.1f}%")
        
        # Duration analysis
        warmup_duration = (self.warmup_df['timestamp'].max() - self.warmup_df['timestamp'].min()).total_seconds()
        warmup_rate = len(self.warmup_df) / warmup_duration if warmup_duration > 0 else 0
        
        print(f"\n📊 THROUGHPUT:")
        print(f"   Duration: {warmup_duration:.1f} seconds")
        print(f"   Processing Rate: {warmup_rate:.1f} tx/second")
        
        return {
            'mean_queue_time': queue_times.mean(),
            'median_queue_time': queue_times.median(),
            'p95_queue_time': queue_times.quantile(0.95),
            'p99_queue_time': queue_times.quantile(0.99),
            'max_queue_time': queue_times.max(),
            'std_queue_time': queue_times.std(),
            'sla_compliance': sla_compliance,
            'processing_rate': warmup_rate,
            'duration': warmup_duration,
            'transaction_count': len(self.warmup_df)
        }
    
    def analyze_post_warmup_period(self):
        """Detailed analysis of post-warmup performance."""
        print(f"\n⚡ POST-WARMUP PERIOD ANALYSIS ({len(self.post_warmup_df):,} transactions)")
        print("=" * 60)
        
        if len(self.post_warmup_df) == 0:
            print("⚠️  No post-warmup data available - system still in warmup")
            return None
        
        queue_times = self.post_warmup_df['queue_time_ms']
        processing_times = self.post_warmup_df['processing_time_ms']
        
        # Timing statistics
        print(f"⏱️  QUEUE TIMING STATISTICS:")
        print(f"   Mean Queue Time: {queue_times.mean():.1f} ms")
        print(f"   Median Queue Time: {queue_times.median():.1f} ms")
        print(f"   P75 Queue Time: {queue_times.quantile(0.75):.1f} ms")
        print(f"   P90 Queue Time: {queue_times.quantile(0.90):.1f} ms")
        print(f"   P95 Queue Time: {queue_times.quantile(0.95):.1f} ms")
        print(f"   P99 Queue Time: {queue_times.quantile(0.99):.1f} ms")
        print(f"   Max Queue Time: {queue_times.max():.1f} ms")
        print(f"   Std Dev: {queue_times.std():.1f} ms")
        
        print(f"\n⚙️  PROCESSING TIMING:")
        print(f"   Mean Processing Time: {processing_times.mean():.2f} ms")
        print(f"   Median Processing Time: {processing_times.median():.2f} ms")
        print(f"   Max Processing Time: {processing_times.max():.2f} ms")
        
        # SLA Analysis
        sla_violations = len(queue_times[queue_times > 100])
        sla_compliance = (1 - sla_violations / len(queue_times)) * 100
        
        print(f"\n🎯 SLA COMPLIANCE (100ms target):")
        print(f"   Violations: {sla_violations:,} transactions ({sla_violations/len(queue_times)*100:.1f}%)")
        print(f"   Compliance: {sla_compliance:.1f}%")
        
        # Duration analysis
        post_warmup_duration = (self.post_warmup_df['timestamp'].max() - self.post_warmup_df['timestamp'].min()).total_seconds()
        post_warmup_rate = len(self.post_warmup_df) / post_warmup_duration if post_warmup_duration > 0 else 0
        
        print(f"\n📊 THROUGHPUT:")
        print(f"   Duration: {post_warmup_duration:.1f} seconds")
        print(f"   Processing Rate: {post_warmup_rate:.1f} tx/second")
        
        return {
            'mean_queue_time': queue_times.mean(),
            'median_queue_time': queue_times.median(),
            'p95_queue_time': queue_times.quantile(0.95),
            'p99_queue_time': queue_times.quantile(0.99),
            'max_queue_time': queue_times.max(),
            'std_queue_time': queue_times.std(),
            'sla_compliance': sla_compliance,
            'processing_rate': post_warmup_rate,
            'duration': post_warmup_duration,
            'transaction_count': len(self.post_warmup_df)
        }
    
    def compare_periods(self, warmup_stats, post_warmup_stats):
        """Compare warmup vs post-warmup performance."""
        print(f"\n🔄 WARMUP vs POST-WARMUP COMPARISON")
        print("=" * 60)
        
        if post_warmup_stats is None:
            print("❌ Cannot compare - no post-warmup data available")
            return
        
        print(f"📊 QUEUE TIME IMPROVEMENTS:")
        
        # Calculate improvements (negative = worse, positive = better)
        mean_improvement = warmup_stats['mean_queue_time'] - post_warmup_stats['mean_queue_time']
        median_improvement = warmup_stats['median_queue_time'] - post_warmup_stats['median_queue_time']
        p95_improvement = warmup_stats['p95_queue_time'] - post_warmup_stats['p95_queue_time']
        p99_improvement = warmup_stats['p99_queue_time'] - post_warmup_stats['p99_queue_time']
        
        print(f"   Mean Queue Time: {warmup_stats['mean_queue_time']:.1f}ms → {post_warmup_stats['mean_queue_time']:.1f}ms")
        print(f"      Improvement: {mean_improvement:+.1f}ms ({mean_improvement/warmup_stats['mean_queue_time']*100:+.1f}%)")
        
        print(f"   Median Queue Time: {warmup_stats['median_queue_time']:.1f}ms → {post_warmup_stats['median_queue_time']:.1f}ms")
        print(f"      Improvement: {median_improvement:+.1f}ms ({median_improvement/warmup_stats['median_queue_time']*100:+.1f}%)")
        
        print(f"   P95 Queue Time: {warmup_stats['p95_queue_time']:.1f}ms → {post_warmup_stats['p95_queue_time']:.1f}ms")
        print(f"      Improvement: {p95_improvement:+.1f}ms ({p95_improvement/warmup_stats['p95_queue_time']*100:+.1f}%)")
        
        print(f"   P99 Queue Time: {warmup_stats['p99_queue_time']:.1f}ms → {post_warmup_stats['p99_queue_time']:.1f}ms")
        print(f"      Improvement: {p99_improvement:+.1f}ms ({p99_improvement/warmup_stats['p99_queue_time']*100:+.1f}%)")
        
        print(f"\n🎯 SLA COMPLIANCE CHANGES:")
        sla_improvement = post_warmup_stats['sla_compliance'] - warmup_stats['sla_compliance']
        print(f"   Warmup SLA: {warmup_stats['sla_compliance']:.1f}%")
        print(f"   Post-Warmup SLA: {post_warmup_stats['sla_compliance']:.1f}%")
        print(f"   Improvement: {sla_improvement:+.1f} percentage points")
        
        print(f"\n📈 THROUGHPUT CHANGES:")
        throughput_improvement = post_warmup_stats['processing_rate'] - warmup_stats['processing_rate']
        print(f"   Warmup Rate: {warmup_stats['processing_rate']:.1f} tx/s")
        print(f"   Post-Warmup Rate: {post_warmup_stats['processing_rate']:.1f} tx/s")
        print(f"   Improvement: {throughput_improvement:+.1f} tx/s ({throughput_improvement/warmup_stats['processing_rate']*100:+.1f}%)")
        
        # Overall assessment
        print(f"\n🏆 OVERALL ASSESSMENT:")
        if mean_improvement > 0 and sla_improvement > 0:
            print("   ✅ SIGNIFICANT IMPROVEMENT: Post-warmup performance better")
        elif mean_improvement > 0 or sla_improvement > 0:
            print("   📈 MODERATE IMPROVEMENT: Some metrics better post-warmup")
        elif abs(mean_improvement) < 50 and abs(sla_improvement) < 5:
            print("   📊 STABLE PERFORMANCE: Minimal difference between periods")
        else:
            print("   ⚠️  PERFORMANCE DEGRADATION: Post-warmup metrics worse")
    
    def analyze_timing_distribution(self):
        """Analyze the distribution of queue times across different periods."""
        print(f"\n📊 QUEUE TIME DISTRIBUTION ANALYSIS")
        print("=" * 60)
        
        # Create time buckets for analysis
        warmup_times = self.warmup_df['queue_time_ms'] if len(self.warmup_df) > 0 else pd.Series()
        post_warmup_times = self.post_warmup_df['queue_time_ms'] if len(self.post_warmup_df) > 0 else pd.Series()
        
        # Define time buckets
        buckets = [
            (0, 50, "🟢 <50ms (Excellent)"),
            (50, 100, "🟡 50-100ms (Good)"),
            (100, 200, "🟠 100-200ms (Poor)"),
            (200, 500, "🔴 200-500ms (Bad)"),
            (500, 1000, "🔴 500ms-1s (Critical)"),
            (1000, float('inf'), "🚨 >1s (Unacceptable)")
        ]
        
        print("WARMUP PERIOD DISTRIBUTION:")
        if len(warmup_times) > 0:
            for min_time, max_time, label in buckets:
                if max_time == float('inf'):
                    count = len(warmup_times[warmup_times >= min_time])
                else:
                    count = len(warmup_times[(warmup_times >= min_time) & (warmup_times < max_time)])
                percentage = (count / len(warmup_times)) * 100
                print(f"   {label}: {count:,} transactions ({percentage:.1f}%)")
        else:
            print("   No warmup data available")
        
        if len(post_warmup_times) > 0:
            print("\nPOST-WARMUP PERIOD DISTRIBUTION:")
            for min_time, max_time, label in buckets:
                if max_time == float('inf'):
                    count = len(post_warmup_times[post_warmup_times >= min_time])
                else:
                    count = len(post_warmup_times[(post_warmup_times >= min_time) & (post_warmup_times < max_time)])
                percentage = (count / len(post_warmup_times)) * 100
                print(f"   {label}: {count:,} transactions ({percentage:.1f}%)")
        else:
            print("\n⚠️  No post-warmup data available for distribution analysis")
    
    def run_complete_analysis(self):
        """Run the complete warmup vs post-warmup analysis."""
        print(f"\n🚀 STARTING COMPLETE WARMUP ANALYSIS")
        print("=" * 70)
        
        # Run individual analyses
        warmup_stats = self.analyze_warmup_period()
        post_warmup_stats = self.analyze_post_warmup_period()
        
        # Compare periods
        self.compare_periods(warmup_stats, post_warmup_stats)
        
        # Distribution analysis
        self.analyze_timing_distribution()
        
        # Summary
        print(f"\n📋 ANALYSIS SUMMARY")
        print("=" * 60)
        print(f"📊 Dataset: {len(self.df):,} total transactions")
        print(f"🔥 Warmup: {len(self.warmup_df):,} transactions ({len(self.warmup_df)/len(self.df)*100:.1f}%)")
        print(f"⚡ Post-Warmup: {len(self.post_warmup_df):,} transactions ({len(self.post_warmup_df)/len(self.df)*100:.1f}%)")
        
        if warmup_stats and post_warmup_stats:
            print(f"🎯 Key Finding: Post-warmup mean queue time {post_warmup_stats['mean_queue_time']:.1f}ms vs warmup {warmup_stats['mean_queue_time']:.1f}ms")
            print(f"📈 SLA Change: {post_warmup_stats['sla_compliance']:.1f}% vs {warmup_stats['sla_compliance']:.1f}%")
        
        return warmup_stats, post_warmup_stats

if __name__ == "__main__":
    # Run the warmup analysis
    analyzer = WarmupQueueAnalyzer('logs/mempool/transaction_timing_analysis_20250603_192458.csv')
    warmup_stats, post_warmup_stats = analyzer.run_complete_analysis() 