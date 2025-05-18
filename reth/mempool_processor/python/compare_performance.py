#!/usr/bin/env python3
"""
Compare performance metrics between batch and non-batch transaction fetching.
"""

import pandas as pd
import sys
import numpy as np
import matplotlib.pyplot as plt
import os

def load_metrics(file_path):
    """Load performance metrics from a CSV file."""
    df = pd.read_csv(file_path)
    # Convert string times to numeric if needed
    if df['fetch_time_ms'].dtype == 'object':
        df['fetch_time_ms'] = pd.to_numeric(df['fetch_time_ms'], errors='coerce')
    if df['simulation_time_ms'].dtype == 'object':
        df['simulation_time_ms'] = pd.to_numeric(df['simulation_time_ms'], errors='coerce')
    if df['total_processing_time_ms'].dtype == 'object':
        df['total_processing_time_ms'] = pd.to_numeric(df['total_processing_time_ms'], errors='coerce')
    return df

def calculate_statistics(df):
    """Calculate detailed statistics for performance metrics."""
    stats = {
        'fetch_time_ms': {
            'mean': df['fetch_time_ms'].mean(),
            'median': df['fetch_time_ms'].median(),
            'std': df['fetch_time_ms'].std(),
            'min': df['fetch_time_ms'].min(),
            'max': df['fetch_time_ms'].max(),
            'p95': np.percentile(df['fetch_time_ms'], 95),
            'p99': np.percentile(df['fetch_time_ms'], 99),
        },
        'simulation_time_ms': {
            'mean': df['simulation_time_ms'].mean(),
            'median': df['simulation_time_ms'].median(),
            'std': df['simulation_time_ms'].std(),
            'min': df['simulation_time_ms'].min(),
            'max': df['simulation_time_ms'].max(),
            'p95': np.percentile(df['simulation_time_ms'], 95),
            'p99': np.percentile(df['simulation_time_ms'], 99),
        },
        'total_processing_time_ms': {
            'mean': df['total_processing_time_ms'].mean(),
            'median': df['total_processing_time_ms'].median(),
            'std': df['total_processing_time_ms'].std(),
            'min': df['total_processing_time_ms'].min(),
            'max': df['total_processing_time_ms'].max(),
            'p95': np.percentile(df['total_processing_time_ms'], 95),
            'p99': np.percentile(df['total_processing_time_ms'], 99),
        }
    }
    
    # Calculate throughput (transactions per second)
    total_time_seconds = df['total_processing_time_ms'].sum() / 1000
    if total_time_seconds > 0:
        stats['throughput'] = len(df) / total_time_seconds
    else:
        stats['throughput'] = 0
        
    return stats

def plot_histograms(batch_df, nonbatch_df, output_dir='.'):
    """Generate histogram plots comparing the two datasets."""
    plt.figure(figsize=(12, 6))
    
    # Fetch Time Histogram
    plt.subplot(1, 3, 1)
    plt.hist(batch_df['fetch_time_ms'], alpha=0.5, bins=50, label='Batch')
    plt.hist(nonbatch_df['fetch_time_ms'], alpha=0.5, bins=50, label='Non-Batch')
    plt.title('Fetch Time (ms)')
    plt.xlabel('Time (ms)')
    plt.ylabel('Frequency')
    plt.legend()
    
    # Simulation Time Histogram
    plt.subplot(1, 3, 2)
    plt.hist(batch_df['simulation_time_ms'], alpha=0.5, bins=50, label='Batch')
    plt.hist(nonbatch_df['simulation_time_ms'], alpha=0.5, bins=50, label='Non-Batch')
    plt.title('Simulation Time (ms)')
    plt.xlabel('Time (ms)')
    plt.legend()
    
    # Total Time Histogram
    plt.subplot(1, 3, 3)
    plt.hist(batch_df['total_processing_time_ms'], alpha=0.5, bins=50, label='Batch')
    plt.hist(nonbatch_df['total_processing_time_ms'], alpha=0.5, bins=50, label='Non-Batch')
    plt.title('Total Processing Time (ms)')
    plt.xlabel('Time (ms)')
    plt.legend()
    
    plt.tight_layout()
    plot_path = os.path.join(output_dir, 'performance_comparison.png')
    plt.savefig(plot_path)
    print(f"Saved performance comparison plot to {plot_path}")

def print_comparison(batch_stats, nonbatch_stats):
    """Print detailed comparison between the two approaches."""
    print("\n=== PERFORMANCE COMPARISON ===\n")
    
    print(f"{'Metric':<25} {'Batch Mode':<15} {'Non-Batch Mode':<15} {'Improvement %':<15}")
    print("-" * 75)
    
    # Fetch Time
    batch_fetch = batch_stats['fetch_time_ms']['mean']
    nonbatch_fetch = nonbatch_stats['fetch_time_ms']['mean']
    improvement = ((nonbatch_fetch - batch_fetch) / nonbatch_fetch) * 100
    print(f"{'Fetch Time (ms)':<25} {batch_fetch:<15.2f} {nonbatch_fetch:<15.2f} {improvement:<15.2f}")
    
    # Simulation Time 
    batch_sim = batch_stats['simulation_time_ms']['mean']
    nonbatch_sim = nonbatch_stats['simulation_time_ms']['mean']
    sim_improvement = ((nonbatch_sim - batch_sim) / nonbatch_sim) * 100 if nonbatch_sim > 0 else 0
    print(f"{'Simulation Time (ms)':<25} {batch_sim:<15.2f} {nonbatch_sim:<15.2f} {sim_improvement:<15.2f}")
    
    # Total Time
    batch_total = batch_stats['total_processing_time_ms']['mean']
    nonbatch_total = nonbatch_stats['total_processing_time_ms']['mean']
    total_improvement = ((nonbatch_total - batch_total) / nonbatch_total) * 100
    print(f"{'Total Processing Time (ms)':<25} {batch_total:<15.2f} {nonbatch_total:<15.2f} {total_improvement:<15.2f}")
    
    # Throughput
    batch_throughput = batch_stats['throughput']
    nonbatch_throughput = nonbatch_stats['throughput']
    throughput_improvement = ((batch_throughput - nonbatch_throughput) / nonbatch_throughput) * 100
    print(f"{'Throughput (tx/sec)':<25} {batch_throughput:<15.2f} {nonbatch_throughput:<15.2f} {throughput_improvement:<15.2f}")
    
    # Additional statistics for fetch time
    print("\n=== DETAILED FETCH TIME STATISTICS ===\n")
    print(f"{'Statistic':<10} {'Batch Mode':<15} {'Non-Batch Mode':<15}")
    print("-" * 45)
    print(f"{'Median':<10} {batch_stats['fetch_time_ms']['median']:<15.2f} {nonbatch_stats['fetch_time_ms']['median']:<15.2f}")
    print(f"{'Std Dev':<10} {batch_stats['fetch_time_ms']['std']:<15.2f} {nonbatch_stats['fetch_time_ms']['std']:<15.2f}")
    print(f"{'Min':<10} {batch_stats['fetch_time_ms']['min']:<15.2f} {nonbatch_stats['fetch_time_ms']['min']:<15.2f}")
    print(f"{'Max':<10} {batch_stats['fetch_time_ms']['max']:<15.2f} {nonbatch_stats['fetch_time_ms']['max']:<15.2f}")
    print(f"{'P95':<10} {batch_stats['fetch_time_ms']['p95']:<15.2f} {nonbatch_stats['fetch_time_ms']['p95']:<15.2f}")
    print(f"{'P99':<10} {batch_stats['fetch_time_ms']['p99']:<15.2f} {nonbatch_stats['fetch_time_ms']['p99']:<15.2f}")
    
    print("\n=== PERFORMANCE SUMMARY ===\n")
    if batch_fetch < nonbatch_fetch:
        print(f"Batch processing is {improvement:.2f}% faster for fetching transactions.")
    else:
        print(f"Non-batch processing is {-improvement:.2f}% faster for fetching transactions.")
    
    if batch_throughput > nonbatch_throughput:
        print(f"Batch processing achieves {throughput_improvement:.2f}% higher throughput.")
    else:
        print(f"Non-batch processing achieves {-throughput_improvement:.2f}% higher throughput.")

def main():
    """Compare performance metrics between two CSV files."""
    if len(sys.argv) < 3:
        print("Usage: python compare_performance.py <batch_csv> <non_batch_csv>")
        sys.exit(1)
    
    batch_file = sys.argv[1]
    non_batch_file = sys.argv[2]
    
    print(f"Analyzing batch mode data from: {batch_file}")
    print(f"Analyzing non-batch mode data from: {non_batch_file}")
    
    # Load metrics
    try:
        batch_metrics = load_metrics(batch_file)
        non_batch_metrics = load_metrics(non_batch_file)
    except Exception as e:
        print(f"Error loading metrics: {e}")
        sys.exit(1)
    
    print(f"Loaded {len(batch_metrics)} batch mode transactions")
    print(f"Loaded {len(non_batch_metrics)} non-batch mode transactions")
    
    # Calculate statistics
    batch_stats = calculate_statistics(batch_metrics)
    nonbatch_stats = calculate_statistics(non_batch_metrics)
    
    # Print comparison
    print_comparison(batch_stats, nonbatch_stats)
    
    # Generate plots
    try:
        plot_histograms(batch_metrics, non_batch_metrics)
    except Exception as e:
        print(f"Could not generate plots: {e}")

if __name__ == "__main__":
    main() 