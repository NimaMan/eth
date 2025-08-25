/// Common aggregation utilities for time-based flow analysis
/// 
/// Provides generic functions to aggregate block-based data into time periods
/// (hourly, daily, weekly) for all entity types.

use std::collections::HashMap;

/// Time period for aggregation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimePeriod {
    Hourly,
    FourHourly,
    Daily,
    Weekly,
    Monthly,
}

impl TimePeriod {
    /// Get number of blocks in this period (assuming 12 second blocks)
    pub fn blocks_per_period(&self) -> u64 {
        match self {
            TimePeriod::Hourly => 300,       // 3600 / 12
            TimePeriod::FourHourly => 1200,  // 14400 / 12
            TimePeriod::Daily => 7200,       // 86400 / 12
            TimePeriod::Weekly => 50400,     // 604800 / 12
            TimePeriod::Monthly => 216000,   // 2592000 / 12 (30 days)
        }
    }
    
    /// Get period name as string
    pub fn as_str(&self) -> &str {
        match self {
            TimePeriod::Hourly => "hourly",
            TimePeriod::FourHourly => "4hourly",
            TimePeriod::Daily => "daily",
            TimePeriod::Weekly => "weekly",
            TimePeriod::Monthly => "monthly",
        }
    }
}

/// Aggregated flow data for a time period
#[derive(Debug, Clone)]
pub struct PeriodFlowData {
    pub period_start_block: u64,
    pub period_end_block: u64,
    pub period_index: usize,
    pub total_inflow: f64,
    pub total_outflow: f64,
    pub net_flow: f64,
    pub transaction_count: u32,
}

/// Generic flow data that can be aggregated
pub trait FlowDataTrait {
    fn get_inflow(&self) -> f64;
    fn get_outflow(&self) -> f64;
    fn get_transaction_count(&self) -> u32;
}

/// Aggregate flow data by time period
pub fn aggregate_flows_by_period<T: FlowDataTrait>(
    flows: Vec<(u64, u64, T)>, // (from_block, to_block, flow_data)
    period: TimePeriod,
) -> Vec<PeriodFlowData> {
    if flows.is_empty() {
        return Vec::new();
    }
    
    // Find the overall range
    let min_block = flows.iter().map(|(from, _, _)| *from).min().unwrap();
    let max_block = flows.iter().map(|(_, to, _)| *to).max().unwrap();
    
    let blocks_per_period = period.blocks_per_period();
    let mut aggregated = Vec::new();
    
    // Create periods
    let mut current_block = min_block;
    let mut period_index = 0;
    
    while current_block < max_block {
        let period_end = std::cmp::min(current_block + blocks_per_period, max_block);
        
        let mut period_inflow = 0.0;
        let mut period_outflow = 0.0;
        let mut period_tx_count = 0;
        
        // Aggregate all flows that overlap with this period
        for (from_block, to_block, flow_data) in &flows {
            // Check if this flow overlaps with current period
            if *from_block < period_end && *to_block > current_block {
                period_inflow += flow_data.get_inflow();
                period_outflow += flow_data.get_outflow();
                period_tx_count += flow_data.get_transaction_count();
            }
        }
        
        aggregated.push(PeriodFlowData {
            period_start_block: current_block,
            period_end_block: period_end,
            period_index,
            total_inflow: period_inflow,
            total_outflow: period_outflow,
            net_flow: period_inflow - period_outflow,
            transaction_count: period_tx_count,
        });
        
        current_block = period_end;
        period_index += 1;
    }
    
    aggregated
}

/// Calculate statistics for flow data
#[derive(Debug, Clone)]
pub struct FlowStatistics {
    pub mean_inflow: f64,
    pub mean_outflow: f64,
    pub median_net_flow: f64,
    pub std_dev_inflow: f64,
    pub std_dev_outflow: f64,
    pub max_inflow: f64,
    pub max_outflow: f64,
    pub total_periods: usize,
}

/// Calculate statistics for period flow data
pub fn calculate_flow_statistics(periods: &[PeriodFlowData]) -> FlowStatistics {
    if periods.is_empty() {
        return FlowStatistics {
            mean_inflow: 0.0,
            mean_outflow: 0.0,
            median_net_flow: 0.0,
            std_dev_inflow: 0.0,
            std_dev_outflow: 0.0,
            max_inflow: 0.0,
            max_outflow: 0.0,
            total_periods: 0,
        };
    }
    
    let n = periods.len() as f64;
    
    // Calculate means
    let sum_inflow: f64 = periods.iter().map(|p| p.total_inflow).sum();
    let sum_outflow: f64 = periods.iter().map(|p| p.total_outflow).sum();
    let mean_inflow = sum_inflow / n;
    let mean_outflow = sum_outflow / n;
    
    // Calculate standard deviations
    let variance_inflow: f64 = periods.iter()
        .map(|p| (p.total_inflow - mean_inflow).powi(2))
        .sum::<f64>() / n;
    let variance_outflow: f64 = periods.iter()
        .map(|p| (p.total_outflow - mean_outflow).powi(2))
        .sum::<f64>() / n;
    
    let std_dev_inflow = variance_inflow.sqrt();
    let std_dev_outflow = variance_outflow.sqrt();
    
    // Calculate median net flow
    let mut net_flows: Vec<f64> = periods.iter().map(|p| p.net_flow).collect();
    net_flows.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_net_flow = if net_flows.len() % 2 == 0 {
        (net_flows[net_flows.len() / 2 - 1] + net_flows[net_flows.len() / 2]) / 2.0
    } else {
        net_flows[net_flows.len() / 2]
    };
    
    // Find maximums
    let max_inflow = periods.iter().map(|p| p.total_inflow).fold(0.0, f64::max);
    let max_outflow = periods.iter().map(|p| p.total_outflow).fold(0.0, f64::max);
    
    FlowStatistics {
        mean_inflow,
        mean_outflow,
        median_net_flow,
        std_dev_inflow,
        std_dev_outflow,
        max_inflow,
        max_outflow,
        total_periods: periods.len(),
    }
}

/// Convert block range to approximate timestamp range (for display)
pub fn blocks_to_time_range(from_block: u64, to_block: u64, block_time_seconds: u64) -> (u64, u64) {
    // Assuming a base timestamp for block 0 (this would need real data in production)
    const GENESIS_TIMESTAMP: u64 = 1438269973; // Ethereum mainnet genesis
    
    let from_timestamp = GENESIS_TIMESTAMP + (from_block * block_time_seconds);
    let to_timestamp = GENESIS_TIMESTAMP + (to_block * block_time_seconds);
    
    (from_timestamp, to_timestamp)
}