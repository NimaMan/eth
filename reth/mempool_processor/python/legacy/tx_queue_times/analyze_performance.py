#!/usr/bin/env python3
"""
Performance Analysis Tool for Scam Detection Service

This script analyzes the processing time performance after the warmup period
by examining logs and metrics from the scam detection service.

Algorithm:
1. Parse transaction fetch timing data from logs
2. Analyze performance trends over time (early vs late execution)
3. Calculate statistics for different phases
4. Identify performance bottlenecks and patterns
5. Generate performance summary report
"""

import re
import json
import sys
from pathlib import Path
from typing import List, Dict, Tuple
import statistics

def parse_fetch_times(log_file: str) -> List[Tuple[str, int]]:
    """Parse transaction fetch times from the log file."""
    times = []
    pattern = r'(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}).*?(\d+) transactions fetched in (\d+)ms'
    
    try:
        with open(log_file, 'r') as f:
            for line in f:
                match = re.search(pattern, line)
                if match:
                    timestamp, tx_count, duration_ms = match.groups()
                    times.append((timestamp, int(duration_ms)))
    except FileNotFoundError:
        print(f"Log file not found: {log_file}")
        return []
    
    return times

def analyze_performance_phases(times: List[Tuple[str, int]]) -> Dict:
    """Analyze performance across different phases."""
    if not times:
        return {}
    
    total_samples = len(times)
    phase_size = total_samples // 3
    
    phases = {
        'early_warmup': times[:phase_size],
        'mid_warmup': times[phase_size:2*phase_size],
        'post_warmup': times[2*phase_size:]
    }
    
    results = {}
    for phase_name, phase_data in phases.items():
        if not phase_data:
            continue
            
        durations = [duration for _, duration in phase_data]
        
        results[phase_name] = {
            'min_ms': min(durations),
            'max_ms': max(durations),
            'avg_ms': round(statistics.mean(durations), 2),
            'median_ms': round(statistics.median(durations), 2),
            'std_dev_ms': round(statistics.stdev(durations) if len(durations) > 1 else 0, 2),
            'samples': len(durations),
            'target_compliance': sum(1 for d in durations if d <= 8) / len(durations) * 100,
            'slow_requests': sum(1 for d in durations if d > 100)
        }
    
    return results

def parse_metrics_file(metrics_file: str) -> Dict:
    """Parse the realtime metrics JSON file."""
    try:
        with open(metrics_file, 'r') as f:
            lines = f.readlines()
            
        # Get the last few metrics entries
        last_metrics = []
        for line in lines[-10:]:
            try:
                metric = json.loads(line.strip())
                last_metrics.append(metric)
            except json.JSONDecodeError:
                continue
                
        if last_metrics:
            latest = last_metrics[-1]
            return {
                'current_tps': latest.get('current_tps', 0),
                'total_processed': latest.get('total_processed', 0),
                'avg_end_to_end_time_ms': latest.get('avg_end_to_end_time_ms', 0),
                'scams_detected': latest.get('scams_detected', 0),
                'service_uptime_seconds': latest.get('service_uptime_seconds', 0),
                'sla_compliance_percentage': latest.get('sla_compliance_percentage', 0),
                'dominant_performance_category': latest.get('dominant_performance_category', 'unknown')
            }
    except FileNotFoundError:
        print(f"Metrics file not found: {metrics_file}")
    
    return {}

def main():
    print("🔍 Scam Detection Service Performance Analysis")
    print("=" * 60)
    
    # File paths
    log_file = "logs/mempool/scam_detection_service_20250603_2131.log"
    metrics_file = "logs/mempool/realtime_metrics_20250603_213157.json"
    
    # Parse timing data
    print("📊 Analyzing transaction fetch performance...")
    times = parse_fetch_times(log_file)
    
    if not times:
        print("❌ No timing data found in logs")
        return
    
    print(f"✅ Found {len(times)} timing samples")
    
    # Analyze performance phases
    phase_analysis = analyze_performance_phases(times)
    
    print("\n📈 Performance Analysis by Phase:")
    print("-" * 50)
    
    for phase_name, stats in phase_analysis.items():
        phase_display = phase_name.replace('_', ' ').title()
        print(f"\n{phase_display} ({stats['samples']} samples):")
        print(f"  Min:          {stats['min_ms']}ms")
        print(f"  Max:          {stats['max_ms']}ms") 
        print(f"  Average:      {stats['avg_ms']}ms")
        print(f"  Median:       {stats['median_ms']}ms")
        print(f"  Std Dev:      {stats['std_dev_ms']}ms")
        print(f"  Target (<8ms): {stats['target_compliance']:.1f}%")
        print(f"  Slow (>100ms): {stats['slow_requests']} requests")
        
        # Performance assessment
        if stats['avg_ms'] <= 10:
            status = "🟢 EXCELLENT"
        elif stats['avg_ms'] <= 25:
            status = "🟡 GOOD"
        elif stats['avg_ms'] <= 50:
            status = "🟠 FAIR"
        else:
            status = "🔴 NEEDS IMPROVEMENT"
        print(f"  Status:       {status}")
    
    # Analyze trends
    print("\n📊 Performance Trends:")
    print("-" * 30)
    
    if len(phase_analysis) >= 2:
        early_avg = phase_analysis['early_warmup']['avg_ms']
        post_avg = phase_analysis['post_warmup']['avg_ms']
        improvement = ((early_avg - post_avg) / early_avg) * 100
        
        if improvement > 5:
            trend = f"🔽 IMPROVED by {improvement:.1f}%"
        elif improvement < -5:
            trend = f"🔼 DEGRADED by {abs(improvement):.1f}%"
        else:
            trend = f"➡️  STABLE ({improvement:+.1f}%)"
        
        print(f"Early → Post Warmup: {trend}")
        print(f"Early Avg: {early_avg}ms → Post Avg: {post_avg}ms")
    
    # Parse latest metrics
    print("\n⚡ Latest Performance Metrics:")
    print("-" * 35)
    
    metrics = parse_metrics_file(metrics_file)
    if metrics:
        print(f"Current TPS:          {metrics['current_tps']:,}")
        print(f"Total Processed:      {metrics['total_processed']:,}")
        print(f"End-to-End Time:      {metrics['avg_end_to_end_time_ms']}ms")
        print(f"Scams Detected:       {metrics['scams_detected']}")
        print(f"Service Uptime:       {metrics['service_uptime_seconds']}s")
        print(f"SLA Compliance:       {metrics['sla_compliance_percentage']}%")
        print(f"Performance Category: {metrics['dominant_performance_category']}")
    
    # Overall assessment
    print("\n🎯 Overall Assessment:")
    print("-" * 25)
    
    if 'post_warmup' in phase_analysis:
        post_stats = phase_analysis['post_warmup']
        if post_stats['avg_ms'] <= 15:
            assessment = "✅ Service is performing well after warmup"
        elif post_stats['avg_ms'] <= 50:
            assessment = "⚠️  Performance is acceptable but could be optimized"
        else:
            assessment = "❌ Performance needs significant improvement"
        
        print(assessment)
        print(f"Post-warmup average: {post_stats['avg_ms']}ms (target: <8ms)")
        
        # Recommendations
        print("\n💡 Recommendations:")
        if post_stats['avg_ms'] > 50:
            print("  • Consider WebSocket connection stability improvements")
            print("  • Optimize RPC batch request sizes")
            print("  • Review network latency to Ethereum node")
        elif post_stats['target_compliance'] < 50:
            print("  • Fine-tune fetch batch sizes for better latency")
            print("  • Consider connection pooling optimizations")
        else:
            print("  • Current performance is acceptable")
            print("  • Monitor for WebSocket connection stability")

if __name__ == "__main__":
    main() 