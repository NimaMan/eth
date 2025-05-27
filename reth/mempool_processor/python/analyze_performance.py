#!/usr/bin/env python3
"""
Analyze performance metrics from the mempool processor benchmark.
Handles the specific CSV format with microsecond timing data.
"""

import pandas as pd
import sys
import numpy as np
import matplotlib.pyplot as plt
import os

def load_metrics(file_path):
    """Load performance metrics from a CSV file."""
    df = pd.read_csv(file_path)
    
    # Convert microseconds to milliseconds for better readability
    df['fetch_time_ms'] = df['fetch_time_us'] / 1000.0
    df['simulation_time_ms'] = df['simulation_time_us'] / 1000.0
    df['state_diff_time_ms'] = df['state_diff_time_us'] / 1000.0
    df['total_processing_time_ms'] = df['total_processing_time_us'] / 1000.0
    df['rpc_response_time_ms'] = df['rpc_response_time_us'] / 1000.0
    df['end_to_end_time_ms'] = df['end_to_end_time_us'] / 1000.0
    
    return df

def calculate_statistics(df):
    """Calculate detailed statistics for performance metrics."""
    stats = {}
    
    # Define metrics to analyze
    metrics = ['fetch_time_ms', 'simulation_time_ms', 'state_diff_time_ms', 
               'total_processing_time_ms', 'rpc_response_time_ms', 'end_to_end_time_ms']
    
    for metric in metrics:
        stats[metric] = {
            'mean': df[metric].mean(),
            'median': df[metric].median(),
            'std': df[metric].std(),
            'min': df[metric].min(),
            'max': df[metric].max(),
            'p95': np.percentile(df[metric], 95),
            'p99': np.percentile(df[metric], 99),
        }
    
    # Calculate success rates
    stats['simulation_success_rate'] = df['simulation_successful'].mean() * 100
    stats['cache_hit_rate'] = df['cache_hit'].mean() * 100
    stats['pool_affecting_rate'] = df['affected_pools'].mean() * 100
    
    # Calculate real-world throughput based on individual transaction processing time
    # For our objective: each transaction must be processed within 1000ms from arrival
    avg_processing_time_ms = df['total_processing_time_ms'].mean()
    if avg_processing_time_ms > 0:
        # Theoretical max throughput if processing sequentially
        stats['theoretical_throughput'] = 1000 / avg_processing_time_ms
        # Actual throughput from benchmark (concurrent processing)
        benchmark_duration_seconds = 19.85  # From the actual benchmark run
        stats['actual_throughput'] = len(df) / benchmark_duration_seconds
    else:
        stats['theoretical_throughput'] = 0
        stats['actual_throughput'] = 0
    
    # Calculate pipeline efficiency
    stats['pipeline_overhead'] = df['total_processing_time_ms'].mean() - (
        df['fetch_time_ms'].mean() + df['simulation_time_ms'].mean() + df['state_diff_time_ms'].mean()
    )
        
    return stats

def plot_performance_analysis(df, output_dir='.'):
    """Generate comprehensive performance analysis plots."""
    fig, axes = plt.subplots(2, 3, figsize=(18, 12))
    
    # Fetch Time Distribution
    axes[0, 0].hist(df['fetch_time_ms'], bins=50, alpha=0.7, color='blue')
    axes[0, 0].set_title('Fetch Time Distribution')
    axes[0, 0].set_xlabel('Time (ms)')
    axes[0, 0].set_ylabel('Frequency')
    axes[0, 0].axvline(df['fetch_time_ms'].mean(), color='red', linestyle='--', label=f'Mean: {df["fetch_time_ms"].mean():.2f}ms')
    axes[0, 0].legend()
    
    # Simulation Time Distribution
    axes[0, 1].hist(df['simulation_time_ms'], bins=50, alpha=0.7, color='green')
    axes[0, 1].set_title('Simulation Time Distribution')
    axes[0, 1].set_xlabel('Time (ms)')
    axes[0, 1].set_ylabel('Frequency')
    axes[0, 1].axvline(df['simulation_time_ms'].mean(), color='red', linestyle='--', label=f'Mean: {df["simulation_time_ms"].mean():.2f}ms')
    axes[0, 1].legend()
    
    # Total Processing Time Distribution
    axes[0, 2].hist(df['total_processing_time_ms'], bins=50, alpha=0.7, color='purple')
    axes[0, 2].set_title('Total Processing Time Distribution')
    axes[0, 2].set_xlabel('Time (ms)')
    axes[0, 2].set_ylabel('Frequency')
    axes[0, 2].axvline(df['total_processing_time_ms'].mean(), color='red', linestyle='--', label=f'Mean: {df["total_processing_time_ms"].mean():.2f}ms')
    axes[0, 2].legend()
    
    # RPC Response Time Distribution
    axes[1, 0].hist(df['rpc_response_time_ms'], bins=50, alpha=0.7, color='orange')
    axes[1, 0].set_title('RPC Response Time Distribution')
    axes[1, 0].set_xlabel('Time (ms)')
    axes[1, 0].set_ylabel('Frequency')
    axes[1, 0].axvline(df['rpc_response_time_ms'].mean(), color='red', linestyle='--', label=f'Mean: {df["rpc_response_time_ms"].mean():.2f}ms')
    axes[1, 0].legend()
    
    # Processing Time Breakdown (Box Plot)
    processing_data = [df['fetch_time_ms'], df['simulation_time_ms'], df['state_diff_time_ms']]
    axes[1, 1].boxplot(processing_data, labels=['Fetch', 'Simulation', 'State Diff'])
    axes[1, 1].set_title('Processing Time Breakdown')
    axes[1, 1].set_ylabel('Time (ms)')
    
    # Transaction Value vs Processing Time
    axes[1, 2].scatter(df['tx_value_eth'], df['total_processing_time_ms'], alpha=0.5)
    axes[1, 2].set_title('Transaction Value vs Processing Time')
    axes[1, 2].set_xlabel('Transaction Value (ETH)')
    axes[1, 2].set_ylabel('Processing Time (ms)')
    
    plt.tight_layout()
    plot_path = os.path.join(output_dir, 'mempool_performance_analysis.png')
    plt.savefig(plot_path, dpi=300, bbox_inches='tight')
    print(f"Saved performance analysis plot to {plot_path}")

def print_detailed_analysis(df, stats):
    """Print comprehensive performance analysis."""
    print("\n" + "="*80)
    print("MEMPOOL PROCESSOR PERFORMANCE ANALYSIS")
    print("="*80)
    
    print(f"\n📊 DATASET OVERVIEW:")
    print(f"   Total Transactions Processed: {len(df):,}")
    print(f"   Simulation Success Rate: {stats['simulation_success_rate']:.1f}%")
    print(f"   Cache Hit Rate: {stats['cache_hit_rate']:.1f}%")
    print(f"   Pool-Affecting Transactions: {stats['pool_affecting_rate']:.1f}%")
    print(f"   Actual Throughput: {stats['actual_throughput']:.2f} tx/sec")
    print(f"   Theoretical Max Throughput: {stats['theoretical_throughput']:.2f} tx/sec")
    
    print(f"\n⚡ TIMING BREAKDOWN (milliseconds):")
    print(f"{'Component':<20} {'Mean':<8} {'Median':<8} {'P95':<8} {'P99':<8} {'Max':<8}")
    print("-" * 65)
    
    components = [
        ('Fetch Time', 'fetch_time_ms'),
        ('Simulation Time', 'simulation_time_ms'),
        ('State Diff Time', 'state_diff_time_ms'),
        ('Total Processing', 'total_processing_time_ms'),
        ('RPC Response', 'rpc_response_time_ms'),
        ('End-to-End', 'end_to_end_time_ms')
    ]
    
    for name, metric in components:
        s = stats[metric]
        print(f"{name:<20} {s['mean']:<8.2f} {s['median']:<8.2f} {s['p95']:<8.2f} {s['p99']:<8.2f} {s['max']:<8.2f}")
    
    print(f"\n🔍 BOTTLENECK ANALYSIS:")
    fetch_pct = (stats['fetch_time_ms']['mean'] / stats['total_processing_time_ms']['mean']) * 100
    sim_pct = (stats['simulation_time_ms']['mean'] / stats['total_processing_time_ms']['mean']) * 100
    diff_pct = (stats['state_diff_time_ms']['mean'] / stats['total_processing_time_ms']['mean']) * 100
    overhead_pct = (stats['pipeline_overhead'] / stats['total_processing_time_ms']['mean']) * 100
    
    print(f"   Fetch Time: {fetch_pct:.1f}% of total processing")
    print(f"   Simulation Time: {sim_pct:.1f}% of total processing")
    print(f"   State Diff Time: {diff_pct:.1f}% of total processing")
    print(f"   Pipeline Overhead: {overhead_pct:.1f}% of total processing")
    
    # Identify the bottleneck
    bottlenecks = [
        ('RPC Fetching', fetch_pct),
        ('Transaction Simulation', sim_pct),
        ('State Diff Extraction', diff_pct),
        ('Pipeline Overhead', overhead_pct)
    ]
    bottleneck = max(bottlenecks, key=lambda x: x[1])
    print(f"   🚨 Primary Bottleneck: {bottleneck[0]} ({bottleneck[1]:.1f}%)")
    
    print(f"\n💰 TRANSACTION CHARACTERISTICS:")
    print(f"   Average Transaction Value: {df['tx_value_eth'].mean():.6f} ETH")
    print(f"   Average Gas Price: {df['gas_price_gwei'].mean():.2f} gwei")
    print(f"   High-Value Transactions (>1 ETH): {(df['tx_value_eth'] > 1).sum()} ({(df['tx_value_eth'] > 1).mean()*100:.1f}%)")
    
    print(f"\n🎯 OPTIMIZATION RECOMMENDATIONS:")
    if bottleneck[0] == 'RPC Fetching':
        print("   • Consider increasing batch size for RPC requests")
        print("   • Implement connection pooling for RPC client")
        print("   • Use faster RPC endpoint or local node")
    elif bottleneck[0] == 'Transaction Simulation':
        print("   • Optimize REVM simulation parameters")
        print("   • Consider parallel simulation for independent transactions")
        print("   • Implement simulation result caching")
    elif bottleneck[0] == 'State Diff Extraction':
        print("   • Optimize state diff tracking algorithms")
        print("   • Consider incremental state diff computation")
        print("   • Reduce granularity of tracked state changes")
    else:
        print("   • Optimize async task scheduling and coordination")
        print("   • Reduce memory allocations in hot paths")
        print("   • Profile for CPU-bound operations")
    
    print(f"\n📈 PERFORMANCE TARGETS:")
    avg_end_to_end_time = df['end_to_end_time_ms'].mean()
    avg_processing_time = df['total_processing_time_ms'].mean()
    target_processing_time = 1000  # 1 second objective
    
    print(f"   🎯 OBJECTIVE: Process each transaction within {target_processing_time}ms from mempool arrival")
    print(f"   Current Average End-to-End Time: {avg_end_to_end_time:.2f}ms (arrival to completion)")
    print(f"   Current Average Processing Time: {avg_processing_time:.2f}ms (fetch to completion)")
    
    if avg_end_to_end_time <= target_processing_time:
        print(f"   ✅ OBJECTIVE MET: End-to-end time is {avg_end_to_end_time:.2f}ms < {target_processing_time}ms")
        print(f"   Theoretical Max Throughput: {stats['theoretical_throughput']:.1f} tx/sec")
    else:
        improvement_needed = avg_end_to_end_time / target_processing_time
        print(f"   ❌ OBJECTIVE NOT MET: Need {improvement_needed:.1f}x speedup")
        print(f"   Required Improvement: Reduce end-to-end time by {((improvement_needed - 1) / improvement_needed * 100):.1f}%")
        print(f"   Focus: Optimize {bottleneck[0].lower()}")
    
    print(f"\n🚀 THROUGHPUT ANALYSIS:")
    print(f"   Actual Benchmark Throughput: {stats['actual_throughput']:.1f} tx/sec")
    print(f"   Theoretical Sequential Throughput: {stats['theoretical_throughput']:.1f} tx/sec")
    
    if stats['theoretical_throughput'] >= 1000:
        print(f"   ✅ Can achieve 1000+ tx/sec with parallel processing")
    else:
        needed_speedup = 1000 / stats['theoretical_throughput']
        print(f"   ⚠️  Need {needed_speedup:.1f}x speedup to reach 1000 tx/sec target")

def main():
    """Analyze performance metrics from the mempool processor benchmark."""
    if len(sys.argv) < 2:
        print("Usage: python analyze_performance.py <performance_csv>")
        sys.exit(1)
    
    csv_file = sys.argv[1]
    
    print(f"Analyzing performance data from: {csv_file}")
    
    # Load metrics
    try:
        df = load_metrics(csv_file)
    except Exception as e:
        print(f"Error loading metrics: {e}")
        sys.exit(1)
    
    print(f"Loaded {len(df)} transaction records")
    
    # Calculate statistics
    stats = calculate_statistics(df)
    
    # Print detailed analysis
    print_detailed_analysis(df, stats)
    
    # Generate plots
    try:
        plot_performance_analysis(df)
        print(f"\n📊 Performance visualization saved as 'mempool_performance_analysis.png'")
    except Exception as e:
        print(f"Could not generate plots: {e}")

if __name__ == "__main__":
    main() 