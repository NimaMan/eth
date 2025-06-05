// Enhanced Transaction Timing for Dual Queue Analysis
// This module extends the existing timing system to properly measure both:
// 1. Our processing queue (lightweight, efficiency monitoring)
// 2. EVM mining queue (detailed, priority queue analysis)

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

/// Enhanced timing structure for dual queue analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualQueueTransactionTiming {
    /// Unique transaction hash
    pub tx_hash: String,
    
    // === OUR PROCESSING QUEUE TIMESTAMPS ===
    /// When we first discovered this transaction (via HTTP polling)
    pub discovery_timestamp_ms: u64,
    /// When transaction entered our internal processing queue
    pub internal_queue_entry_timestamp_ms: u64,
    /// When we started processing this transaction
    pub processing_start_timestamp_ms: u64,
    /// When we finished processing this transaction
    pub processing_end_timestamp_ms: u64,
    
    // === EVM MINING QUEUE DATA ===
    /// Estimated mempool arrival time (discovery - estimated propagation delay)
    pub estimated_mempool_entry_ms: u64,
    /// Transaction nonce (for dependency analysis)
    pub nonce: Option<u64>,
    /// From address (for sender analysis)
    pub from_address: String,
    /// To address (for destination analysis)
    pub to_address: Option<String>,
    
    // === GAS PRICING DATA (Priority Queue Analysis) ===
    /// Legacy gas price (Wei) - for priority ordering
    pub gas_price_wei: String,
    /// Gas limit requested
    pub gas_limit: u64,
    /// EIP-1559 max fee per gas (Wei) - if applicable
    pub max_fee_per_gas_wei: Option<String>,
    /// EIP-1559 max priority fee per gas (Wei) - if applicable  
    pub max_priority_fee_per_gas_wei: Option<String>,
    /// Transaction type (0=Legacy, 1=EIP-2930, 2=EIP-1559)
    pub transaction_type: u8,
    /// Transaction value in Wei
    pub tx_value_wei: String,
    
    // === MINING OUTCOME DATA (Populated by Python post-processing) ===
    /// When transaction was actually mined (block timestamp)
    pub mining_timestamp_ms: Option<u64>,
    /// Block number where transaction was included
    pub block_number: Option<u64>,
    /// Position within the block (transaction index)
    pub transaction_index_in_block: Option<u64>,
    /// Actual gas used during execution
    pub gas_used: Option<u64>,
    /// Effective gas price paid (after EIP-1559 calculation)
    pub effective_gas_price_wei: Option<String>,
    /// Whether transaction was successful
    pub mining_status: Option<String>, // "success", "failed", "pending", "dropped"
    
    // === DERIVED METRICS (Calculated after mining data collection) ===
    
    // Our Processing Queue Metrics
    /// Time in our internal queue (lightweight tracking)
    pub our_queue_time_us: u64,
    /// Our total processing time (discovery to completion)
    pub our_processing_time_us: u64,
    
    // EVM Mining Queue Metrics  
    /// Total time from mempool entry to mining (EVM queue time)
    pub evm_queue_time_ms: Option<u64>,
    /// Gas price percentile within the block (priority effectiveness)
    pub gas_price_percentile_in_block: Option<f64>,
    /// Gas price percentile within mempool at time of discovery
    pub gas_price_percentile_in_mempool: Option<f64>,
    /// Estimated mining probability based on gas price
    pub mining_probability_score: Option<f64>,
    
    // === PERFORMANCE CLASSIFICATION ===
    /// Our processing performance category
    pub our_processing_category: String, // "excellent", "good", "acceptable", "poor"
    /// EVM queue performance relative to gas price tier
    pub evm_queue_category: Option<String>, // "fast", "normal", "slow", "dropped"
    /// Whether our processing met SLA
    pub our_sla_compliance: bool,
    /// Whether EVM queue time was reasonable for gas price paid
    pub evm_queue_efficiency: Option<bool>,
}

impl DualQueueTransactionTiming {
    /// Create new timing tracker with discovery timestamp
    pub fn new(tx_hash: String, from_address: String, to_address: Option<String>) -> Self {
        let now_ms = current_timestamp_ms();
        Self {
            tx_hash,
            discovery_timestamp_ms: now_ms,
            internal_queue_entry_timestamp_ms: now_ms,
            processing_start_timestamp_ms: 0,
            processing_end_timestamp_ms: 0,
            
            // Estimate mempool entry (discovery - propagation delay)
            estimated_mempool_entry_ms: now_ms.saturating_sub(500), // 500ms propagation estimate
            
            from_address,
            to_address,
            nonce: None,
            
            // Gas data (populated from transaction)
            gas_price_wei: "0".to_string(),
            gas_limit: 21000,
            max_fee_per_gas_wei: None,
            max_priority_fee_per_gas_wei: None,
            transaction_type: 0,
            tx_value_wei: "0".to_string(),
            
            // Mining data (populated later)
            mining_timestamp_ms: None,
            block_number: None,
            transaction_index_in_block: None,
            gas_used: None,
            effective_gas_price_wei: None,
            mining_status: None,
            
            // Derived metrics (calculated later)
            our_queue_time_us: 0,
            our_processing_time_us: 0,
            evm_queue_time_ms: None,
            gas_price_percentile_in_block: None,
            gas_price_percentile_in_mempool: None,
            mining_probability_score: None,
            
            // Performance classification
            our_processing_category: "excellent".to_string(),
            evm_queue_category: None,
            our_sla_compliance: true,
            evm_queue_efficiency: None,
        }
    }
    
    /// Mark start of our internal processing
    pub fn start_processing(&mut self) {
        self.processing_start_timestamp_ms = current_timestamp_ms();
    }
    
    /// Mark completion of our processing and calculate metrics
    pub fn finish_processing(&mut self) {
        self.processing_end_timestamp_ms = current_timestamp_ms();
        
        // Calculate our processing metrics
        if self.processing_start_timestamp_ms > self.internal_queue_entry_timestamp_ms {
            self.our_queue_time_us = 
                (self.processing_start_timestamp_ms - self.internal_queue_entry_timestamp_ms) * 1000;
        }
        
        if self.processing_end_timestamp_ms > self.discovery_timestamp_ms {
            self.our_processing_time_us = 
                (self.processing_end_timestamp_ms - self.discovery_timestamp_ms) * 1000;
        }
        
        // Classify our processing performance
        let processing_ms = self.our_processing_time_us as f64 / 1000.0;
        self.our_processing_category = if processing_ms <= 1.0 {
            "excellent".to_string()
        } else if processing_ms <= 10.0 {
            "good".to_string()
        } else if processing_ms <= 100.0 {
            "acceptable".to_string()
        } else {
            "poor".to_string()
        };
        
        // Check SLA compliance (100ms target)
        self.our_sla_compliance = processing_ms <= 100.0;
    }
    
    /// Update mining data (called by Python post-processing)
    pub fn update_mining_data(
        &mut self,
        mining_timestamp_ms: u64,
        block_number: u64,
        transaction_index: u64,
        gas_used: u64,
        effective_gas_price_wei: String,
        status: String,
    ) {
        self.mining_timestamp_ms = Some(mining_timestamp_ms);
        self.block_number = Some(block_number);
        self.transaction_index_in_block = Some(transaction_index);
        self.gas_used = Some(gas_used);
        self.effective_gas_price_wei = Some(effective_gas_price_wei);
        self.mining_status = Some(status);
        
        // Calculate EVM queue time
        if mining_timestamp_ms > self.estimated_mempool_entry_ms {
            self.evm_queue_time_ms = Some(mining_timestamp_ms - self.estimated_mempool_entry_ms);
        }
    }
    
    /// Calculate gas price analytics (called during post-processing)
    pub fn calculate_gas_price_analytics(
        &mut self,
        block_gas_prices: &[u64],
        mempool_gas_prices: &[u64],
    ) {
        let our_gas_price = self.gas_price_wei.parse::<u64>().unwrap_or(0);
        
        // Calculate percentile within block
        if !block_gas_prices.is_empty() {
            let higher_count = block_gas_prices.iter().filter(|&&price| price > our_gas_price).count();
            self.gas_price_percentile_in_block = Some(
                (block_gas_prices.len() - higher_count) as f64 / block_gas_prices.len() as f64 * 100.0
            );
        }
        
        // Calculate percentile within mempool at discovery time
        if !mempool_gas_prices.is_empty() {
            let higher_count = mempool_gas_prices.iter().filter(|&&price| price > our_gas_price).count();
            self.gas_price_percentile_in_mempool = Some(
                (mempool_gas_prices.len() - higher_count) as f64 / mempool_gas_prices.len() as f64 * 100.0
            );
        }
        
        // Estimate mining probability based on gas price percentile
        if let Some(percentile) = self.gas_price_percentile_in_mempool {
            self.mining_probability_score = Some(percentile / 100.0);
        }
        
        // Classify EVM queue performance
        if let Some(queue_time) = self.evm_queue_time_ms {
            let queue_seconds = queue_time as f64 / 1000.0;
            self.evm_queue_category = Some(
                if queue_seconds <= 15.0 {
                    "fast".to_string()        // Faster than 1 block
                } else if queue_seconds <= 60.0 {
                    "normal".to_string()      // 1-4 blocks
                } else if queue_seconds <= 300.0 {
                    "slow".to_string()        // 5-20 blocks
                } else {
                    "very_slow".to_string()   // 20+ blocks
                }
            );
            
            // Check efficiency relative to gas price
            if let Some(gas_percentile) = self.gas_price_percentile_in_mempool {
                // High gas price should lead to fast mining
                let expected_fast = gas_percentile > 75.0;
                let actually_fast = queue_seconds <= 30.0;
                self.evm_queue_efficiency = Some(expected_fast == actually_fast);
            }
        }
    }
    
    /// Generate summary for logging
    pub fn summary(&self) -> String {
        let evm_queue_str = match self.evm_queue_time_ms {
            Some(time) => format!("{:.1}s", time as f64 / 1000.0),
            None => "pending".to_string(),
        };
        
        format!(
            "Our: {:.1}ms ({}) | EVM: {} | Gas: {} Gwei | Status: {}",
            self.our_processing_time_us as f64 / 1000.0,
            self.our_processing_category,
            evm_queue_str,
            self.gas_price_wei.parse::<u64>().unwrap_or(0) / 1_000_000_000,
            self.mining_status.as_ref().unwrap_or(&"pending".to_string())
        )
    }
}

/// Dual queue performance metrics with 100K warmup
pub struct DualQueuePerformanceMetrics {
    /// Total transactions processed
    pub total_processed: u64,
    /// Warmup phase transaction count
    pub warmup_count: u64,
    /// Whether warmup is complete
    pub warmup_complete: bool,
    
    /// Lightweight tracking of our processing (always active)
    pub our_recent_queue_times: std::collections::VecDeque<u64>, // microseconds
    pub our_recent_processing_times: std::collections::VecDeque<u64>, // microseconds
    
    /// Detailed EVM queue tracking (post-warmup only)
    pub pending_evm_analysis: HashMap<String, DualQueueTransactionTiming>,
    
    /// Performance statistics
    pub our_avg_queue_time_us: u64,
    pub our_avg_processing_time_us: u64,
    pub our_sla_compliance_rate: f64,
    
    /// Data export files
    pub our_metrics_file: std::fs::File,
    pub evm_queue_file: std::fs::File,
}

impl DualQueuePerformanceMetrics {
    pub fn new() -> eyre::Result<Self> {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        
        // Lightweight our-processing metrics (always active)
        let our_metrics_path = format!(
            "/home/nima/code/crypto/logs/mempool/our_processing_metrics_{}.csv", 
            timestamp
        );
        
        // Detailed EVM queue analysis (post-warmup only)
        let evm_queue_path = format!(
            "/home/nima/code/crypto/logs/mempool/evm_queue_analysis_{}.csv", 
            timestamp
        );
        
        let mut our_file = std::fs::File::create(&our_metrics_path)?;
        let mut evm_file = std::fs::File::create(&evm_queue_path)?;
        
        // Write headers
        writeln!(our_file, "tx_hash,discovery_timestamp_ms,processing_time_us,queue_time_us,sla_compliance,performance_category")?;
        writeln!(evm_file, "tx_hash,estimated_mempool_entry_ms,mining_timestamp_ms,gas_price_wei,gas_limit,transaction_type,from_address,to_address,nonce,block_number,transaction_index,gas_used,effective_gas_price_wei,mining_status,evm_queue_time_ms,gas_price_percentile_in_block,gas_price_percentile_in_mempool,mining_probability_score,evm_queue_category,evm_queue_efficiency")?;
        
        Ok(Self {
            total_processed: 0,
            warmup_count: 0,
            warmup_complete: false,
            our_recent_queue_times: std::collections::VecDeque::with_capacity(1000),
            our_recent_processing_times: std::collections::VecDeque::with_capacity(1000),
            pending_evm_analysis: HashMap::new(),
            our_avg_queue_time_us: 0,
            our_avg_processing_time_us: 0,
            our_sla_compliance_rate: 100.0,
            our_metrics_file: our_file,
            evm_queue_file: evm_file,
        })
    }
    
    pub fn track_transaction(&mut self, timing: &DualQueueTransactionTiming) {
        self.total_processed += 1;
        
        // Always track our processing performance (lightweight)
        self.track_our_processing(timing);
        
        // Check warmup completion
        if !self.warmup_complete {
            self.warmup_count += 1;
            if self.warmup_count >= 100_000 {
                self.warmup_complete = true;
                eprintln!("🔥 WARMUP COMPLETE: Starting detailed EVM queue analysis after {} transactions", self.warmup_count);
            }
        }
        
        // Track EVM queue only after warmup
        if self.warmup_complete {
            self.track_evm_queue(timing);
        }
    }
    
    fn track_our_processing(&mut self, timing: &DualQueueTransactionTiming) {
        // Update running averages for our processing
        self.our_recent_queue_times.push_back(timing.our_queue_time_us);
        self.our_recent_processing_times.push_back(timing.our_processing_time_us);
        
        // Keep only recent 1000 measurements
        if self.our_recent_queue_times.len() > 1000 {
            self.our_recent_queue_times.pop_front();
        }
        if self.our_recent_processing_times.len() > 1000 {
            self.our_recent_processing_times.pop_front();
        }
        
        // Calculate averages
        if !self.our_recent_processing_times.is_empty() {
            self.our_avg_processing_time_us = 
                self.our_recent_processing_times.iter().sum::<u64>() / self.our_recent_processing_times.len() as u64;
        }
        
        if !self.our_recent_queue_times.is_empty() {
            self.our_avg_queue_time_us = 
                self.our_recent_queue_times.iter().sum::<u64>() / self.our_recent_queue_times.len() as u64;
        }
        
        // Calculate SLA compliance rate
        let compliant_count = self.our_recent_processing_times.iter()
            .filter(|&&time| time <= 100_000) // 100ms in microseconds
            .count();
        self.our_sla_compliance_rate = 
            compliant_count as f64 / self.our_recent_processing_times.len() as f64 * 100.0;
        
        // Log to our processing metrics file
        if let Err(e) = writeln!(
            self.our_metrics_file,
            "{},{},{},{},{},{}",
            timing.tx_hash,
            timing.discovery_timestamp_ms,
            timing.our_processing_time_us,
            timing.our_queue_time_us,
            timing.our_sla_compliance,
            timing.our_processing_category
        ) {
            eprintln!("Failed to write our processing metrics: {}", e);
        }
    }
    
    fn track_evm_queue(&mut self, timing: &DualQueueTransactionTiming) {
        // Store for mining analysis
        self.pending_evm_analysis.insert(timing.tx_hash.clone(), timing.clone());
        
        // Periodically export batch for Python mining data collection
        if self.pending_evm_analysis.len() >= 500 {
            self.export_evm_batch_for_mining_analysis();
        }
    }
    
    fn export_evm_batch_for_mining_analysis(&mut self) {
        // Export current batch to JSON for Python processing
        let batch: Vec<_> = self.pending_evm_analysis.values().cloned().collect();
        
        let export_path = format!(
            "/home/nima/code/crypto/logs/mempool/evm_batch_export_{}.json",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );
        
        if let Ok(json) = serde_json::to_string_pretty(&batch) {
            if let Err(e) = std::fs::write(&export_path, json) {
                eprintln!("Failed to export EVM batch: {}", e);
            } else {
                eprintln!("📤 Exported {} transactions for EVM mining analysis: {}", batch.len(), export_path);
            }
        }
        
        // Clear pending (Python will handle mining data collection)
        self.pending_evm_analysis.clear();
    }
    
    pub fn get_our_processing_summary(&self) -> String {
        format!(
            "Our Queue: Avg {:.2}ms | Processing: {:.2}ms | SLA: {:.1}% | Warmup: {}/100K {}",
            self.our_avg_queue_time_us as f64 / 1000.0,
            self.our_avg_processing_time_us as f64 / 1000.0,
            self.our_sla_compliance_rate,
            self.warmup_count,
            if self.warmup_complete { "✅" } else { "🔄" }
        )
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// Re-export for use in main service
pub use DualQueueTransactionTiming as TransactionTiming;
pub use DualQueuePerformanceMetrics as PerformanceMetrics;