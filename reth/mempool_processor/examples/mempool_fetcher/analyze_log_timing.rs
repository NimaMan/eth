/// Analyze Timing Patterns from Transaction Logs
/// 
/// This tool analyzes the transaction hash log files to visualize:
/// - Transaction arrival patterns over time
/// - Burst detection and intervals
/// - Latency distribution
/// - Time gaps between transactions
///
/// Usage:
///   cargo run --example analyze_log_timing -- /path/to/tx_hashes_1000tx_20250708_113140.txt

use std::fs::File;
use std::io::{BufRead, BufReader};
use chrono::{DateTime, Local};
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(name = "analyze_log_timing")]
struct Args {
    /// Path to the transaction hash log file
    log_file: String,
}

#[derive(Debug)]
struct TransactionEntry {
    hash: String,
    detection_ns: u64,
    timestamp: DateTime<Local>,
    elapsed_sec: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("📊 Analyzing Transaction Log Timing: {}", args.log_file);
    println!("=" .repeat(80));
    
    let file = File::open(&args.log_file)?;
    let reader = BufReader::new(file);
    
    let mut transactions = Vec::new();
    let mut first_timestamp: Option<DateTime<Local>> = None;
    
    // Parse the log file
    for line in reader.lines() {
        let line = line?;
        
        // Skip comments and empty lines
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        
        // Parse transaction entry: hash | detection_ns | timestamp | elapsed_sec
        let parts: Vec<&str> = line.split(" | ").collect();
        if parts.len() >= 3 {
            let hash = parts[0].trim().to_string();
            let detection_ns = parts[1].trim()
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let timestamp_str = parts[2].trim();
            let elapsed_sec = if parts.len() >= 4 {
                parts[3].trim()
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            
            if let Ok(timestamp) = DateTime::parse_from_str(
                &format!("{} +00:00", timestamp_str),
                "%Y-%m-%d %H:%M:%S%.3f %z"
            ) {
                let timestamp = timestamp.with_timezone(&Local);
                if first_timestamp.is_none() {
                    first_timestamp = Some(timestamp);
                }
                
                transactions.push(TransactionEntry {
                    hash,
                    detection_ns,
                    timestamp,
                    elapsed_sec,
                });
            }
        }
    }
    
    if transactions.is_empty() {
        println!("❌ No transactions found in log file");
        return Ok(());
    }
    
    println!("✅ Found {} transactions", transactions.len());
    println!();
    
    // Analyze timing patterns
    println!("⏱️  TIMING ANALYSIS");
    println!("-" .repeat(80));
    
    // Calculate inter-arrival times
    let mut inter_arrival_times = Vec::new();
    for i in 1..transactions.len() {
        let time_diff = transactions[i].timestamp
            .signed_duration_since(transactions[i-1].timestamp)
            .num_microseconds()
            .unwrap_or(0) as f64 / 1000.0; // Convert to milliseconds
        inter_arrival_times.push(time_diff);
    }
    
    // Detect bursts (transactions arriving within 1ms of each other)
    let mut bursts = Vec::new();
    let mut current_burst = vec![0];
    
    for (i, &time_diff) in inter_arrival_times.iter().enumerate() {
        if time_diff < 1.0 { // Within 1ms
            current_burst.push(i + 1);
        } else {
            if current_burst.len() > 1 {
                bursts.push(current_burst.clone());
            }
            current_burst = vec![i + 1];
        }
    }
    if current_burst.len() > 1 {
        bursts.push(current_burst);
    }
    
    // Print burst analysis
    println!("🎯 BURST PATTERNS:");
    println!("   Total bursts detected: {}", bursts.len());
    
    if !bursts.is_empty() {
        let burst_sizes: Vec<usize> = bursts.iter().map(|b| b.len()).collect();
        let avg_burst_size = burst_sizes.iter().sum::<usize>() as f64 / burst_sizes.len() as f64;
        let max_burst_size = burst_sizes.iter().max().unwrap_or(&0);
        let min_burst_size = burst_sizes.iter().min().unwrap_or(&0);
        
        println!("   Burst sizes: min={}, avg={:.1}, max={}", min_burst_size, avg_burst_size, max_burst_size);
        println!();
        
        // Show first few bursts
        println!("   First 5 bursts:");
        for (i, burst) in bursts.iter().take(5).enumerate() {
            let burst_start = &transactions[burst[0]];
            let burst_end = &transactions[burst[burst.len()-1]];
            let duration = burst_end.timestamp
                .signed_duration_since(burst_start.timestamp)
                .num_microseconds()
                .unwrap_or(0) as f64 / 1000.0;
            
            println!("     Burst {}: {} transactions in {:.3}ms at t={:.3}s",
                i + 1,
                burst.len(),
                duration,
                burst_start.elapsed_sec
            );
        }
    }
    
    // Analyze gaps between bursts
    if bursts.len() > 1 {
        println!();
        println!("⏳ INTER-BURST GAPS:");
        
        let mut gaps = Vec::new();
        for i in 1..bursts.len() {
            let prev_burst_last = bursts[i-1].last().unwrap();
            let curr_burst_first = bursts[i].first().unwrap();
            
            let gap = transactions[*curr_burst_first].timestamp
                .signed_duration_since(transactions[*prev_burst_last].timestamp)
                .num_milliseconds() as f64;
            gaps.push(gap);
        }
        
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p50_gap = gaps[gaps.len() / 2];
        let p90_gap = gaps[gaps.len() * 9 / 10];
        let min_gap = gaps.first().unwrap_or(&0.0);
        let max_gap = gaps.last().unwrap_or(&0.0);
        
        println!("   Gap statistics (ms):");
        println!("     Min: {:.1}ms", min_gap);
        println!("     P50: {:.1}ms", p50_gap);
        println!("     P90: {:.1}ms", p90_gap);
        println!("     Max: {:.1}ms", max_gap);
    }
    
    // Detection latency analysis
    println!();
    println!("🔍 DETECTION LATENCY ANALYSIS:");
    
    let mut detection_latencies: Vec<u64> = transactions.iter()
        .map(|t| t.detection_ns)
        .collect();
    detection_latencies.sort();
    
    let p50_detection = detection_latencies[detection_latencies.len() / 2];
    let p90_detection = detection_latencies[detection_latencies.len() * 9 / 10];
    let p99_detection = detection_latencies[detection_latencies.len() * 99 / 100];
    let min_detection = detection_latencies.first().unwrap_or(&0);
    let max_detection = detection_latencies.last().unwrap_or(&0);
    
    println!("   Detection latency (socket read time):");
    println!("     Min: {}ns ({:.3}μs)", min_detection, *min_detection as f64 / 1000.0);
    println!("     P50: {}ns ({:.3}μs)", p50_detection, p50_detection as f64 / 1000.0);
    println!("     P90: {}ns ({:.3}μs)", p90_detection, p90_detection as f64 / 1000.0);
    println!("     P99: {}ns ({:.3}μs)", p99_detection, p99_detection as f64 / 1000.0);
    println!("     Max: {}ns ({:.3}μs)", max_detection, *max_detection as f64 / 1000.0);
    
    // Overall timeline
    println!();
    println!("📅 TIMELINE SUMMARY:");
    let total_duration = transactions.last().unwrap().elapsed_sec;
    let avg_rate = transactions.len() as f64 / total_duration;
    
    println!("   Total duration: {:.3} seconds", total_duration);
    println!("   Average rate: {:.1} tx/sec", avg_rate);
    println!("   Start time: {}", transactions.first().unwrap().timestamp.format("%Y-%m-%d %H:%M:%S.%3f"));
    println!("   End time: {}", transactions.last().unwrap().timestamp.format("%Y-%m-%d %H:%M:%S.%3f"));
    
    Ok(())
}