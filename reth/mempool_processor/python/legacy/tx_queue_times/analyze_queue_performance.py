#!/usr/bin/env python3
"""
Transaction Processing Queue Analysis

OBJECTIVE: Analyze the mempool transaction processing system as a queuing theory problem
to understand performance characteristics, bottlenecks, and optimization opportunities.

QUEUING SYSTEM MODEL:
- Arrival Process: Transactions arriving in mempool
- Service Process: Our scam detection processing
- Queue: Transactions waiting to be processed
- Servers: Our processing capacity

ALGORITHM:
1. Load transaction timing data from CSV
2. Calculate arrival rates (λ) and service rates (μ)
3. Analyze queue length distribution
4. Compute system utilization (ρ = λ/μ)
5. Examine wait time distributions
6. Identify performance bottlenecks
7. Generate recommendations for optimization
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
from datetime import datetime, timedelta
import warnings
warnings.filterwarnings('ignore')

class TransactionQueueAnalyzer:
    def __init__(self, csv_file_path):
        """Initialize the queue analyzer with transaction timing data."""
        self.df = pd.read_csv(csv_file_path)
        self.df['timestamp'] = pd.to_datetime(self.df['timestamp'])
        
        # Convert timing columns to proper types
        self.df['queue_time_ms'] = pd.to_numeric(self.df['queue_time_ms'])
        self.df['processing_time_ms'] = pd.to_numeric(self.df['processing_time_ms'])
        
        print(f"📊 Loaded {len(self.df):,} transaction records")
        print(f"📅 Time range: {self.df['timestamp'].min()} to {self.df['timestamp'].max()}")
        print(f"⏱️  Duration: {(self.df['timestamp'].max() - self.df['timestamp'].min()).total_seconds():.1f} seconds")
        
    def analyze_arrival_process(self):
        """Analyze the transaction arrival patterns."""
        print("\n🚀 ARRIVAL PROCESS ANALYSIS")
        print("=" * 50)
        
        # Calculate inter-arrival times
        self.df = self.df.sort_values('timestamp')
        self.df['inter_arrival_time'] = self.df['timestamp'].diff().dt.total_seconds()
        
        # Remove first row (no previous arrival)
        inter_arrivals = self.df['inter_arrival_time'].dropna()
        
        # Arrival rate (transactions per second)
        total_time = (self.df['timestamp'].max() - self.df['timestamp'].min()).total_seconds()
        arrival_rate = len(self.df) / total_time
        
        print(f"📈 Arrival Rate (λ): {arrival_rate:.2f} transactions/second")
        print(f"📈 Arrival Rate (λ): {arrival_rate * 60:.1f} transactions/minute")
        print(f"📈 Arrival Rate (λ): {arrival_rate * 3600:.0f} transactions/hour")
        
        print(f"\n⏰ Inter-arrival Times:")
        print(f"   Mean: {inter_arrivals.mean():.3f} seconds")
        print(f"   Median: {inter_arrivals.median():.3f} seconds")
        print(f"   Std Dev: {inter_arrivals.std():.3f} seconds")
        print(f"   Min: {inter_arrivals.min():.3f} seconds")
        print(f"   Max: {inter_arrivals.max():.3f} seconds")
        
        # Check if arrivals follow Poisson process (exponential inter-arrivals)
        print(f"\n🔬 Poisson Process Check:")
        print(f"   Expected mean = std dev for exponential: {inter_arrivals.mean():.3f} vs {inter_arrivals.std():.3f}")
        print(f"   Ratio (should be ~1 for Poisson): {inter_arrivals.std() / inter_arrivals.mean():.2f}")
        
        return arrival_rate, inter_arrivals
    
    def analyze_service_process(self):
        """Analyze the service (processing) characteristics."""
        print("\n⚙️  SERVICE PROCESS ANALYSIS")
        print("=" * 50)
        
        # Service times (processing time)
        service_times = self.df['processing_time_ms'] / 1000.0  # Convert to seconds
        
        # Service rate (transactions per second we can process)
        mean_service_time = service_times.mean()
        service_rate = 1 / mean_service_time if mean_service_time > 0 else float('inf')
        
        print(f"⚡ Service Rate (μ): {service_rate:.2f} transactions/second")
        print(f"⚡ Service Rate (μ): {service_rate * 60:.1f} transactions/minute")
        print(f"⚡ Service Rate (μ): {service_rate * 3600:.0f} transactions/hour")
        
        print(f"\n⏱️  Service Times:")
        print(f"   Mean: {mean_service_time:.4f} seconds ({mean_service_time * 1000:.1f} ms)")
        print(f"   Median: {service_times.median():.4f} seconds ({service_times.median() * 1000:.1f} ms)")
        print(f"   P95: {service_times.quantile(0.95):.4f} seconds ({service_times.quantile(0.95) * 1000:.1f} ms)")
        print(f"   P99: {service_times.quantile(0.99):.4f} seconds ({service_times.quantile(0.99) * 1000:.1f} ms)")
        print(f"   Max: {service_times.max():.4f} seconds ({service_times.max() * 1000:.1f} ms)")
        
        return service_rate, service_times
    
    def analyze_queue_performance(self, arrival_rate, service_rate):
        """Analyze overall queue system performance."""
        print("\n📊 QUEUE SYSTEM PERFORMANCE")
        print("=" * 50)
        
        # System utilization (ρ = λ/μ)
        utilization = arrival_rate / service_rate if service_rate > 0 else float('inf')
        
        print(f"🎯 System Utilization (ρ = λ/μ): {utilization:.4f} ({utilization * 100:.2f}%)")
        
        if utilization >= 1.0:
            print("🚨 WARNING: System is overloaded! (ρ ≥ 1)")
            print("   Queue will grow indefinitely - need more processing capacity")
        elif utilization > 0.8:
            print("⚠️  WARNING: High utilization (ρ > 0.8)")
            print("   System approaching saturation - consider scaling")
        elif utilization > 0.6:
            print("✅ GOOD: Moderate utilization (0.6 < ρ ≤ 0.8)")
            print("   System handling load well with some buffer")
        else:
            print("✅ EXCELLENT: Low utilization (ρ ≤ 0.6)")
            print("   System has plenty of spare capacity")
        
        # Queue length analysis (from wait times)
        queue_times = self.df['queue_time_ms'] / 1000.0  # Convert to seconds
        
        print(f"\n📏 Queue Wait Times:")
        print(f"   Mean: {queue_times.mean():.3f} seconds ({queue_times.mean() * 1000:.1f} ms)")
        print(f"   Median: {queue_times.median():.3f} seconds ({queue_times.median() * 1000:.1f} ms)")
        print(f"   P95: {queue_times.quantile(0.95):.3f} seconds ({queue_times.quantile(0.95) * 1000:.1f} ms)")
        print(f"   P99: {queue_times.quantile(0.99):.3f} seconds ({queue_times.quantile(0.99) * 1000:.1f} ms)")
        print(f"   Max: {queue_times.max():.3f} seconds ({queue_times.max() * 1000:.1f} ms)")
        
        # Theoretical M/M/1 queue metrics (if applicable)
        if 0 < utilization < 1:
            print(f"\n🔬 M/M/1 Queue Theory Predictions:")
            expected_queue_length = utilization**2 / (1 - utilization)
            expected_wait_time = utilization / (service_rate * (1 - utilization))
            expected_response_time = 1 / (service_rate * (1 - utilization))
            
            print(f"   Expected Queue Length: {expected_queue_length:.2f} transactions")
            print(f"   Expected Wait Time: {expected_wait_time:.3f} seconds ({expected_wait_time * 1000:.1f} ms)")
            print(f"   Expected Response Time: {expected_response_time:.3f} seconds ({expected_response_time * 1000:.1f} ms)")
            
            # Compare with actual
            actual_avg_wait = queue_times.mean()
            print(f"\n📊 Theory vs Reality:")
            print(f"   Predicted Wait: {expected_wait_time * 1000:.1f} ms")
            print(f"   Actual Wait: {actual_avg_wait * 1000:.1f} ms")
            print(f"   Ratio: {actual_avg_wait / expected_wait_time:.2f}x theoretical")
        
        return utilization, queue_times
    
    def analyze_pool_transactions(self):
        """Special analysis for pool transactions."""
        print("\n🏊 POOL TRANSACTION ANALYSIS")
        print("=" * 50)
        
        pool_txs = self.df[self.df['is_pool_tx'] == True]
        
        if len(pool_txs) == 0:
            print("⚠️  No pool transactions found in dataset")
            return
        
        print(f"📊 Pool Transactions: {len(pool_txs):,} ({len(pool_txs)/len(self.df)*100:.1f}% of total)")
        
        pool_queue_times = pool_txs['queue_time_ms'] / 1000.0
        pool_processing_times = pool_txs['processing_time_ms'] / 1000.0
        
        print(f"\n⏱️  Pool Transaction Timing:")
        print(f"   Avg Queue Time: {pool_queue_times.mean() * 1000:.1f} ms")
        print(f"   Avg Processing Time: {pool_processing_times.mean() * 1000:.1f} ms")
        print(f"   P95 Queue Time: {pool_queue_times.quantile(0.95) * 1000:.1f} ms")
        print(f"   P95 Processing Time: {pool_processing_times.quantile(0.95) * 1000:.1f} ms")
        
        # Scam detection rate
        scam_txs = pool_txs[pool_txs['scam_detected'] == True]
        print(f"\n🚨 Scam Detection:")
        print(f"   Scams Found: {len(scam_txs):,}")
        print(f"   Scam Rate: {len(scam_txs)/len(pool_txs)*100:.2f}% of pool transactions")
    
    def generate_recommendations(self, utilization, arrival_rate, service_rate):
        """Generate performance optimization recommendations."""
        print("\n💡 PERFORMANCE RECOMMENDATIONS")
        print("=" * 50)
        
        if utilization >= 1.0:
            print("🚨 CRITICAL - Immediate Action Required:")
            required_service_rate = arrival_rate * 1.2  # 20% buffer
            print(f"   - Need service rate of {required_service_rate:.2f} tx/s (currently {service_rate:.2f})")
            print(f"   - Consider adding {(required_service_rate/service_rate):.1f}x more processing capacity")
            print("   - Implement load balancing or horizontal scaling")
            
        elif utilization > 0.8:
            print("⚠️  HIGH LOAD - Scale Soon:")
            buffer_service_rate = arrival_rate * 1.3  # 30% buffer
            print(f"   - Recommend service rate of {buffer_service_rate:.2f} tx/s for safety")
            print("   - Monitor queue lengths closely")
            print("   - Prepare scaling plan")
            
        elif utilization > 0.6:
            print("✅ HEALTHY - Monitor & Optimize:")
            print("   - System performing well")
            print("   - Focus on service time optimization")
            print("   - Monitor for traffic growth")
            
        else:
            print("✅ EXCELLENT - Optimize for Latency:")
            print("   - System has spare capacity")
            print("   - Focus on reducing individual transaction latency")
            print("   - Consider real-time optimizations")
        
        print(f"\n🎯 100ms SLA Analysis:")
        queue_times_ms = self.df['queue_time_ms']
        sla_violations = len(queue_times_ms[queue_times_ms > 100])
        sla_compliance = (1 - sla_violations / len(queue_times_ms)) * 100
        
        print(f"   SLA Compliance: {sla_compliance:.1f}%")
        print(f"   Violations: {sla_violations:,} transactions > 100ms")
        
        if sla_compliance < 95:
            print("   🚨 SLA not met - need faster processing")
        elif sla_compliance < 99:
            print("   ⚠️  SLA marginal - monitor closely")
        else:
            print("   ✅ SLA excellent - maintaining target")
    
    def run_full_analysis(self):
        """Run complete queuing system analysis."""
        print("🎯 TRANSACTION PROCESSING QUEUE ANALYSIS")
        print("=" * 60)
        print(f"📊 Dataset: {len(self.df):,} transactions")
        print(f"🏊 Pool transactions: {len(self.df[self.df['is_pool_tx'] == True]):,}")
        print(f"🚨 Scam detections: {len(self.df[self.df['scam_detected'] == True]):,}")
        
        # Run all analyses
        arrival_rate, inter_arrivals = self.analyze_arrival_process()
        service_rate, service_times = self.analyze_service_process()
        utilization, queue_times = self.analyze_queue_performance(arrival_rate, service_rate)
        self.analyze_pool_transactions()
        self.generate_recommendations(utilization, arrival_rate, service_rate)
        
        print(f"\n📈 SUMMARY METRICS:")
        print(f"   Arrival Rate: {arrival_rate:.2f} tx/s")
        print(f"   Service Rate: {service_rate:.2f} tx/s") 
        print(f"   Utilization: {utilization:.1%}")
        print(f"   Avg Queue Time: {queue_times.mean() * 1000:.1f} ms")
        print(f"   P95 Queue Time: {queue_times.quantile(0.95) * 1000:.1f} ms")

if __name__ == "__main__":
    # Analyze the current timing data
    analyzer = TransactionQueueAnalyzer('logs/mempool/transaction_timing_analysis_20250603_192458.csv')
    analyzer.run_full_analysis() 