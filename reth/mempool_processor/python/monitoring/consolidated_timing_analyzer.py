#!/usr/bin/env python3
"""
CONSOLIDATED TRANSACTION TIMING ANALYZER

OBJECTIVE: 
Measure and analyze the complete transaction processing pipeline from mempool 
arrival to final processing completion. This tool provides comprehensive timing 
analysis for the Ethereum mempool processor scam detection system.

FEATURES:
- Load and validate transaction timing data from CSV logs
- Analyze timing phases: mempool residence, queue time, processing stages
- Calculate performance metrics and SLA compliance
- Identify bottlenecks and optimization opportunities  
- Generate detailed reports and visualizations
- Support for multiple data files and time ranges

USAGE:
    python consolidated_timing_analyzer.py [--file path/to/timing.csv] [--output reports/]

CODE REFERENCE: 
    Rust timing generation: rust/mempool_processor/src/bin/scam_detection_service.rs:139-211
    CSV logging format: rust/mempool_processor/src/bin/scam_detection_service.rs:510-587

MEASUREMENT PHASES:
1. Mempool Residence: Time transaction spends in Ethereum mempool before detection
2. Internal Queue: Time waiting in our processing queue
3. Pool Check: Database lookup time for pool address validation
4. REVM Simulation: EVM transaction simulation time
5. State Analysis: State difference calculation time
6. Scam Detection: ML-based scam detection inference time
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
import argparse
import json
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple
import warnings
warnings.filterwarnings('ignore')

class ConsolidatedTimingAnalyzer:
    """Comprehensive transaction timing analysis tool."""
    
    # CSV column definitions from Rust scam_detection_service.rs
    TIMING_COLUMNS = {
        'mempool_residence_time_us': 'Mempool Residence',
        'internal_queue_time_us': 'Internal Queue', 
        'pool_check_time_us': 'Pool Check',
        'revm_simulation_time_us': 'REVM Simulation',
        'state_analysis_time_us': 'State Analysis',
        'scam_detection_time_us': 'Scam Detection',
        'total_processing_time_us': 'Total Processing',
        'end_to_end_time_us': 'End-to-End'
    }
    
    # Performance categories from Rust implementation
    PERFORMANCE_CATEGORIES = ['excellent', 'good', 'acceptable', 'poor']
    
    def __init__(self, csv_file_path: str, output_dir: str = "./reports"):
        """Initialize analyzer with timing data file."""
        self.csv_file = Path(csv_file_path)
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(exist_ok=True)
        
        print(f"🚀 CONSOLIDATED TRANSACTION TIMING ANALYZER")
        print("=" * 80)
        print(f"📂 Data File: {self.csv_file}")
        print(f"📊 Output Directory: {self.output_dir}")
        
        # Load and validate data
        self.df = self._load_and_validate_data()
        if self.df is None:
            raise ValueError("Failed to load valid timing data")
            
        self.analysis_results = {}
        
    def _load_and_validate_data(self) -> Optional[pd.DataFrame]:
        """Load CSV data and validate required columns."""
        try:
            print(f"📖 Loading timing data...")
            df = pd.read_csv(self.csv_file)
            
            # Validate required columns exist
            required_cols = list(self.TIMING_COLUMNS.keys()) + [
                'tx_hash', 'is_pool_transaction', 'scam_detected', 
                'sla_violation', 'performance_category'
            ]
            
            missing_cols = [col for col in required_cols if col not in df.columns]
            if missing_cols:
                print(f"❌ Missing required columns: {missing_cols}")
                return None
                
            # Convert timing columns to numeric
            for col in self.TIMING_COLUMNS.keys():
                df[col] = pd.to_numeric(df[col], errors='coerce')
                
            # Calculate derived metrics
            df = self._calculate_derived_metrics(df)
            
            print(f"✅ Loaded {len(df):,} transaction records")
            print(f"📊 Data Range: {df.index[0]} to {df.index[-1]}")
            print(f"⏱️  Time Span: {len(df)} transactions")
            
            return df
            
        except FileNotFoundError:
            print(f"❌ File not found: {self.csv_file}")
            return None
        except Exception as e:
            print(f"❌ Error loading data: {e}")
            return None
    
    def _calculate_derived_metrics(self, df: pd.DataFrame) -> pd.DataFrame:
        """Calculate additional metrics from timing data."""
        # Convert microseconds to milliseconds for analysis
        for col in self.TIMING_COLUMNS.keys():
            df[f"{col.replace('_us', '_ms')}"] = df[col] / 1000.0
            
        # Calculate ratios and percentages
        df['processing_efficiency'] = (
            df['total_processing_time_us'] / 
            (df['end_to_end_time_us'] + 1)  # +1 to avoid division by zero
        )
        
        df['queue_vs_processing_ratio'] = (
            df['internal_queue_time_us'] / 
            (df['total_processing_time_us'] + 1)
        )
        
        return df
    
    def analyze_timing_phases(self) -> Dict:
        """Analyze each timing phase with comprehensive statistics."""
        print(f"\n📊 TIMING PHASE ANALYSIS")
        print("=" * 60)
        
        results = {}
        
        for col, phase_name in self.TIMING_COLUMNS.items():
            if col not in self.df.columns:
                continue
                
            # Convert to milliseconds for readability
            data_ms = self.df[col] / 1000.0
            
            # Filter out zero values for meaningful statistics
            data_nonzero = data_ms[data_ms > 0]
            
            if len(data_nonzero) == 0:
                continue
                
            stats = {
                'count': len(data_nonzero),
                'mean_ms': data_nonzero.mean(),
                'median_ms': data_nonzero.median(),
                'std_ms': data_nonzero.std(),
                'min_ms': data_nonzero.min(),
                'max_ms': data_nonzero.max(),
                'p25_ms': data_nonzero.quantile(0.25),
                'p75_ms': data_nonzero.quantile(0.75),
                'p95_ms': data_nonzero.quantile(0.95),
                'p99_ms': data_nonzero.quantile(0.99),
                'zero_count': len(data_ms[data_ms == 0])
            }
            
            results[phase_name] = stats
            
            print(f"\n⏱️  {phase_name.upper()}:")
            print(f"   Samples: {stats['count']:,} non-zero + {stats['zero_count']:,} zero")
            print(f"   Mean:    {stats['mean_ms']:.3f}ms")
            print(f"   Median:  {stats['median_ms']:.3f}ms")
            print(f"   P95:     {stats['p95_ms']:.3f}ms")
            print(f"   P99:     {stats['p99_ms']:.3f}ms")
            print(f"   Max:     {stats['max_ms']:.3f}ms")
            
        return results
    
    def analyze_transaction_categories(self) -> Dict:
        """Analyze performance by transaction categories."""
        print(f"\n📊 TRANSACTION CATEGORY ANALYSIS")
        print("=" * 60)
        
        total_txs = len(self.df)
        results = {}
        
        # Pool transactions
        pool_txs = self.df[self.df['is_pool_transaction'] == True]
        results['pool_transactions'] = {
            'count': len(pool_txs),
            'percentage': len(pool_txs) / total_txs * 100,
            'avg_processing_ms': pool_txs['total_processing_time_us'].mean() / 1000
        }
        
        # Scam transactions  
        scam_txs = self.df[self.df['scam_detected'] == True]
        results['scam_transactions'] = {
            'count': len(scam_txs),
            'percentage': len(scam_txs) / total_txs * 100,
            'avg_processing_ms': scam_txs['total_processing_time_us'].mean() / 1000
        }
        
        # Performance categories
        perf_categories = {}
        for category in self.PERFORMANCE_CATEGORIES:
            cat_txs = self.df[self.df['performance_category'] == category]
            if len(cat_txs) > 0:
                perf_categories[category] = {
                    'count': len(cat_txs),
                    'percentage': len(cat_txs) / total_txs * 100,
                    'avg_end_to_end_ms': cat_txs['end_to_end_time_us'].mean() / 1000
                }
        results['performance_categories'] = perf_categories
        
        # SLA compliance
        sla_violations = self.df[self.df['sla_violation'] == True]
        results['sla_compliance'] = {
            'violations': len(sla_violations),
            'compliance_rate': (total_txs - len(sla_violations)) / total_txs * 100,
            'avg_violation_time_ms': sla_violations['end_to_end_time_us'].mean() / 1000 if len(sla_violations) > 0 else 0
        }
        
        # Print results
        print(f"\n🏊 Pool Transactions: {results['pool_transactions']['count']:,} ({results['pool_transactions']['percentage']:.1f}%)")
        print(f"🚨 Scam Transactions: {results['scam_transactions']['count']:,} ({results['scam_transactions']['percentage']:.1f}%)")
        
        print(f"\n⚡ Performance Categories:")
        for cat, stats in perf_categories.items():
            print(f"   {cat.capitalize()}: {stats['count']:,} ({stats['percentage']:.1f}%)")
            
        print(f"\n📊 SLA Compliance: {results['sla_compliance']['compliance_rate']:.1f}%")
        print(f"   Violations: {results['sla_compliance']['violations']:,}")
        
        return results
    
    def identify_bottlenecks(self) -> Dict:
        """Identify system bottlenecks by analyzing timing contributions."""
        print(f"\n🎯 BOTTLENECK ANALYSIS") 
        print("=" * 60)
        
        # Calculate average contribution of each phase
        phase_averages = {}
        for col, phase_name in self.TIMING_COLUMNS.items():
            if col in self.df.columns:
                avg_ms = self.df[col].mean() / 1000.0
                phase_averages[phase_name] = avg_ms
                
        # Sort by contribution
        sorted_phases = sorted(phase_averages.items(), key=lambda x: x[1], reverse=True)
        total_time = sum(phase_averages.values())
        
        results = {
            'total_avg_time_ms': total_time,
            'phase_contributions': {}
        }
        
        print(f"\n📊 LATENCY CONTRIBUTION BREAKDOWN:")
        print(f"   Total Average Latency: {total_time:.3f}ms")
        print()
        
        for i, (phase, time_ms) in enumerate(sorted_phases, 1):
            percentage = (time_ms / total_time) * 100 if total_time > 0 else 0
            results['phase_contributions'][phase] = {
                'time_ms': time_ms,
                'percentage': percentage,
                'rank': i
            }
            print(f"   {i}. {phase}: {time_ms:.3f}ms ({percentage:.1f}%)")
            
        # Identify primary bottleneck and recommendations
        if sorted_phases:
            primary_bottleneck = sorted_phases[0][0]
            primary_percentage = (sorted_phases[0][1] / total_time) * 100
            
            print(f"\n🎯 PRIMARY BOTTLENECK: {primary_bottleneck}")
            print(f"   Contributes {primary_percentage:.1f}% of total latency")
            
            # Provide optimization recommendations
            recommendations = {
                'Mempool Residence': "Integrate private mempool feeds (Flashbots, Eden)",
                'Internal Queue': "Increase processing parallelism or optimize queue management",
                'REVM Simulation': "Optimize REVM configuration or implement state caching",
                'Pool Check': "Implement pool address caching or async lookups",
                'State Analysis': "Optimize state diff calculation algorithms",
                'Scam Detection': "Optimize ML model inference or implement model caching"
            }
            
            recommendation = recommendations.get(primary_bottleneck, "Profile and optimize implementation")
            print(f"   💡 RECOMMENDATION: {recommendation}")
            
            results['primary_bottleneck'] = {
                'phase': primary_bottleneck,
                'percentage': primary_percentage,
                'recommendation': recommendation
            }
            
        return results
    
    def calculate_throughput_metrics(self) -> Dict:
        """Calculate transaction throughput and processing capacity."""
        print(f"\n📈 THROUGHPUT & CAPACITY ANALYSIS")
        print("=" * 60)
        
        total_transactions = len(self.df)
        
        # Estimate time range (using transaction count as proxy for time)
        # In a real implementation, we'd use actual timestamps
        estimated_duration_minutes = total_transactions / 1000  # Rough estimate
        
        # Processing capacity analysis
        avg_processing_ms = self.df['total_processing_time_us'].mean() / 1000.0
        theoretical_tps = 1000 / avg_processing_ms if avg_processing_ms > 0 else 0
        
        # Queue efficiency
        avg_queue_ms = self.df['internal_queue_time_us'].mean() / 1000.0
        queue_efficiency = (avg_processing_ms / (avg_queue_ms + avg_processing_ms)) * 100 if (avg_queue_ms + avg_processing_ms) > 0 else 0
        
        results = {
            'total_transactions': total_transactions,
            'estimated_duration_min': estimated_duration_minutes,
            'avg_processing_time_ms': avg_processing_ms,
            'avg_queue_time_ms': avg_queue_ms,
            'theoretical_max_tps': theoretical_tps,
            'queue_efficiency_pct': queue_efficiency
        }
        
        print(f"📊 PROCESSING METRICS:")
        print(f"   Total Transactions: {total_transactions:,}")
        print(f"   Avg Processing Time: {avg_processing_ms:.3f}ms")
        print(f"   Avg Queue Time: {avg_queue_ms:.3f}ms")
        print(f"   Theoretical Max TPS: {theoretical_tps:.0f}")
        print(f"   Queue Efficiency: {queue_efficiency:.1f}%")
        
        return results
    
    def generate_visualizations(self):
        """Generate comprehensive timing visualizations."""
        print(f"\n📊 GENERATING VISUALIZATIONS")
        print("=" * 60)
        
        # Set up plotting style
        plt.style.use('seaborn-v0_8')
        sns.set_palette("husl")
        
        # Create comprehensive timing analysis plots
        fig = plt.figure(figsize=(20, 16))
        gs = fig.add_gridspec(4, 3, hspace=0.3, wspace=0.3)
        
        # 1. Timing phase distributions (top row)
        timing_phases = ['mempool_residence_time_us', 'internal_queue_time_us', 'total_processing_time_us']
        for i, col in enumerate(timing_phases):
            ax = fig.add_subplot(gs[0, i])
            if col in self.df.columns:
                data_ms = self.df[col] / 1000.0
                data_filtered = data_ms[data_ms <= data_ms.quantile(0.99)]  # Remove outliers
                ax.hist(data_filtered, bins=50, alpha=0.7, edgecolor='black')
                ax.set_xlabel('Time (ms)')
                ax.set_ylabel('Frequency')
                ax.set_title(f'{self.TIMING_COLUMNS.get(col, col)}\nMean: {data_ms.mean():.2f}ms')
                ax.grid(True, alpha=0.3)
        
        # 2. Performance category distribution (second row, left)
        ax = fig.add_subplot(gs[1, 0])
        perf_counts = self.df['performance_category'].value_counts()
        colors = ['green', 'yellow', 'orange', 'red'][:len(perf_counts)]
        ax.pie(perf_counts.values, labels=perf_counts.index, autopct='%1.1f%%', colors=colors)
        ax.set_title('Performance Categories')
        
        # 3. SLA compliance over time (second row, middle)
        ax = fig.add_subplot(gs[1, 1])
        window_size = len(self.df) // 20  # 20 windows
        compliance_rates = []
        for i in range(0, len(self.df) - window_size, window_size):
            window_data = self.df.iloc[i:i+window_size]
            compliance = (window_data['sla_violation'] == False).mean() * 100
            compliance_rates.append(compliance)
        
        ax.plot(range(len(compliance_rates)), compliance_rates, marker='o')
        ax.set_xlabel('Time Window')
        ax.set_ylabel('SLA Compliance (%)')
        ax.set_title('SLA Compliance Over Time')
        ax.grid(True, alpha=0.3)
        ax.axhline(y=95, color='red', linestyle='--', alpha=0.7, label='95% Target')
        ax.legend()
        
        # 4. Bottleneck analysis (second row, right)
        ax = fig.add_subplot(gs[1, 2])
        phase_averages = {}
        for col, phase_name in self.TIMING_COLUMNS.items():
            if col in self.df.columns:
                avg_ms = self.df[col].mean() / 1000.0
                if avg_ms > 0:
                    phase_averages[phase_name] = avg_ms
        
        if phase_averages:
            phases = list(phase_averages.keys())
            times = list(phase_averages.values())
            ax.bar(range(len(phases)), times, alpha=0.7)
            ax.set_xticks(range(len(phases)))
            ax.set_xticklabels(phases, rotation=45, ha='right')
            ax.set_ylabel('Average Time (ms)')
            ax.set_title('Average Timing by Phase')
            ax.grid(True, alpha=0.3)
        
        # 5. End-to-end timing scatter plot (third row, spans all)
        ax = fig.add_subplot(gs[2, :])
        sample_size = min(5000, len(self.df))  # Sample for performance
        sample_df = self.df.sample(n=sample_size)
        
        scatter = ax.scatter(
            sample_df['total_processing_time_us'] / 1000,
            sample_df['end_to_end_time_us'] / 1000,
            c=sample_df['sla_violation'],
            cmap='RdYlGn_r',
            alpha=0.6,
            s=20
        )
        ax.set_xlabel('Processing Time (ms)')
        ax.set_ylabel('End-to-End Time (ms)')
        ax.set_title(f'Processing vs End-to-End Time (Sample of {sample_size:,} transactions)')
        ax.grid(True, alpha=0.3)
        plt.colorbar(scatter, ax=ax, label='SLA Violation')
        
        # 6. Detailed timing breakdown (fourth row)
        detailed_phases = ['pool_check_time_us', 'revm_simulation_time_us', 'state_analysis_time_us', 'scam_detection_time_us']
        for i, col in enumerate(detailed_phases):
            if i < 3 and col in self.df.columns:  # Only first 3 phases for space
                ax = fig.add_subplot(gs[3, i])
                data_ms = self.df[col] / 1000.0
                data_nonzero = data_ms[data_ms > 0]
                if len(data_nonzero) > 0:
                    ax.boxplot([data_nonzero], labels=[self.TIMING_COLUMNS.get(col, col)])
                    ax.set_ylabel('Time (ms)')
                    ax.set_title(f'{self.TIMING_COLUMNS.get(col, col)}\nMedian: {data_nonzero.median():.3f}ms')
                    ax.grid(True, alpha=0.3)
        
        plt.suptitle('Comprehensive Transaction Timing Analysis', fontsize=16, fontweight='bold')
        
        # Save the plot
        plot_path = self.output_dir / 'comprehensive_timing_analysis.png'
        plt.savefig(plot_path, dpi=300, bbox_inches='tight')
        print(f"📊 Comprehensive analysis plot saved: {plot_path}")
        
        plt.show()
    
    def generate_summary_report(self):
        """Generate a comprehensive summary report."""
        print(f"\n📋 GENERATING SUMMARY REPORT")
        print("=" * 60)
        
        # Collect all analysis results
        timing_results = self.analyze_timing_phases()
        category_results = self.analyze_transaction_categories()
        bottleneck_results = self.identify_bottlenecks()
        throughput_results = self.calculate_throughput_metrics()
        
        # Save detailed results to JSON
        full_results = {
            'metadata': {
                'analysis_timestamp': datetime.now().isoformat(),
                'data_file': str(self.csv_file),
                'total_transactions': len(self.df),
            },
            'timing_phases': timing_results,
            'transaction_categories': category_results,
            'bottleneck_analysis': bottleneck_results,
            'throughput_metrics': throughput_results
        }
        
        json_path = self.output_dir / 'timing_analysis_results.json'
        with open(json_path, 'w') as f:
            json.dump(full_results, f, indent=2, default=str)
        print(f"📄 Detailed results saved: {json_path}")
        
        # Generate CSV summary for easy consumption
        summary_data = []
        for phase, stats in timing_results.items():
            summary_data.append({
                'Phase': phase,
                'Mean_ms': stats['mean_ms'],
                'Median_ms': stats['median_ms'],
                'P95_ms': stats['p95_ms'],
                'Max_ms': stats['max_ms'],
                'Sample_Count': stats['count']
            })
        
        summary_df = pd.DataFrame(summary_data)
        csv_path = self.output_dir / 'timing_phase_summary.csv'
        summary_df.to_csv(csv_path, index=False)
        print(f"📈 Phase summary CSV saved: {csv_path}")
        
        # Print executive summary
        print(f"\n🎯 EXECUTIVE SUMMARY")
        print("=" * 60)
        
        primary_bottleneck = bottleneck_results.get('primary_bottleneck', {})
        sla_compliance = category_results.get('sla_compliance', {})
        
        print(f"📊 Analyzed {len(self.df):,} transactions")
        print(f"⏱️  Primary Bottleneck: {primary_bottleneck.get('phase', 'Unknown')} ({primary_bottleneck.get('percentage', 0):.1f}% of latency)")
        print(f"📈 SLA Compliance: {sla_compliance.get('compliance_rate', 0):.1f}%")
        print(f"🚀 Theoretical Max TPS: {throughput_results.get('theoretical_max_tps', 0):.0f}")
        print(f"⚡ Processing Efficiency: {throughput_results.get('queue_efficiency_pct', 0):.1f}%")
        
        if primary_bottleneck.get('recommendation'):
            print(f"\n💡 KEY RECOMMENDATION: {primary_bottleneck['recommendation']}")
        
        return full_results

def main():
    """Main execution function with command line interface."""
    parser = argparse.ArgumentParser(description='Consolidated Transaction Timing Analyzer')
    parser.add_argument('--file', '-f', 
                       help='Path to transaction timing CSV file')
    parser.add_argument('--output', '-o', default='./reports',
                       help='Output directory for reports and visualizations')
    parser.add_argument('--no-plots', action='store_true',
                       help='Skip generating plots (for headless environments)')
    
    args = parser.parse_args()
    
    # Auto-detect timing file if not provided
    if not args.file:
        possible_paths = [
            '/home/nima/code/crypto/logs/mempool/transaction_timing_analysis_20250603_222244.csv',
            '/home/nima/code/crypto/logs/mempool/transaction_timing_analysis_20250603_213157.csv',
        ]
        
        for path in possible_paths:
            if Path(path).exists():
                args.file = path
                break
                
        if not args.file:
            print("❌ No timing data file found. Please specify with --file option.")
            print("Expected locations:")
            for path in possible_paths:
                print(f"   {path}")
            return
    
    try:
        # Initialize analyzer
        analyzer = ConsolidatedTimingAnalyzer(args.file, args.output)
        
        # Run comprehensive analysis
        results = analyzer.generate_summary_report()
        
        # Generate visualizations (if not disabled)
        if not args.no_plots:
            analyzer.generate_visualizations()
        
        print(f"\n✅ ANALYSIS COMPLETE")
        print(f"📂 Reports saved to: {analyzer.output_dir}")
        
    except Exception as e:
        print(f"❌ Analysis failed: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()