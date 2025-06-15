// TODO: Update to use DebugTraceCallStateDiffCalculator instead
// use crate::tx_simulator::{ComprehensiveStateChange};
use super::python_bridge::{PythonResult, PythonStateChange};

// Temporary placeholder until we update to use DebugTraceCallStateDiffCalculator
type ComprehensiveStateChange = std::collections::HashMap<String, serde_json::Value>;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use tracing::info;
use ethers::types::Address;
use std::str::FromStr;

/// Comparison result for a single address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressComparison {
    pub address: String,
    pub rust_token_net: i64,
    pub rust_denom_net: f64,
    pub python_token_net: i64,
    pub python_denom_net: f64,
    pub token_matches: bool,
    pub denom_matches: bool,
    pub token_diff: i64,
    pub denom_diff: f64,
    pub status: ComparisonStatus,
}

/// Overall comparison status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonStatus {
    ExactMatch,
    WithinTolerance,
    SignificantDifference,
    OnlyInRust,
    OnlyInPython,
}

/// Complete comparison result for a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionComparison {
    pub transaction_hash: String,
    pub block_number: u64,
    pub rust_success: bool,
    pub python_success: bool,
    pub address_comparisons: Vec<AddressComparison>,
    pub summary: ComparisonSummary,
    pub rust_processing_time_ms: u64,
    pub python_processing_time_ms: u64,
}

/// Summary statistics for a comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonSummary {
    pub total_addresses: usize,
    pub exact_matches: usize,
    pub within_tolerance: usize,
    pub significant_differences: usize,
    pub only_in_rust: usize,
    pub only_in_python: usize,
    pub overall_match: bool,
}

/// Configuration for comparison tolerances
#[derive(Debug, Clone)]
pub struct ComparisonConfig {
    pub denom_tolerance: f64,
    pub token_tolerance: i64,
    pub require_exact_addresses: bool,
    pub ignore_zero_changes: bool,
}

impl Default for ComparisonConfig {
    fn default() -> Self {
        Self {
            denom_tolerance: 1e-9,  // Very small tolerance for floating point
            token_tolerance: 0,     // Exact match required for token amounts
            require_exact_addresses: false, // Allow different address sets
            ignore_zero_changes: true,      // Ignore addresses with zero changes
        }
    }
}

/// Engine for comparing Rust and Python state change results
pub struct ComparisonEngine {
    config: ComparisonConfig,
}

impl ComparisonEngine {
    pub fn new(config: ComparisonConfig) -> Self {
        Self { config }
    }

    /// Compare Rust and Python results for a single transaction
    pub fn compare_transaction(
        &self,
        rust_changes: &HashMap<String, ComprehensiveStateChange>,
        python_result: &PythonResult,
        rust_processing_time_ms: u64,
    ) -> TransactionComparison {
        info!("Comparing results for transaction {}", python_result.transaction_hash);

        // Convert Rust results to comparable format
        let rust_map = self.rust_changes_to_map(rust_changes);
        let python_map = self.python_changes_to_map(&python_result.state_changes);

        // Get all unique addresses
        let mut all_addresses: std::collections::HashSet<String> = std::collections::HashSet::new();
        all_addresses.extend(rust_map.keys().cloned());
        all_addresses.extend(python_map.keys().cloned());

        // Compare each address
        let mut address_comparisons = Vec::new();
        for address in all_addresses {
            let comparison = self.compare_address(&address, &rust_map, &python_map);
            
            // Skip zero changes if configured
            if self.config.ignore_zero_changes && self.is_zero_change(&comparison) {
                continue;
            }
            
            address_comparisons.push(comparison);
        }

        // Generate summary
        let summary = self.generate_summary(&address_comparisons);

        TransactionComparison {
            transaction_hash: python_result.transaction_hash.clone(),
            block_number: python_result.block_number,
            rust_success: true, // Assume success if we got results
            python_success: python_result.success,
            address_comparisons,
            summary,
            rust_processing_time_ms,
            python_processing_time_ms: python_result.processing_time_ms,
        }
    }

    /// Convert Rust state changes to a map for comparison
    fn rust_changes_to_map(&self, changes: &HashMap<String, ComprehensiveStateChange>) -> HashMap<String, (i64, f64)> {
        let mut map = HashMap::new();
        
        for (address, change) in changes {
            // Address is already a string, ensure it's checksummed
            let checksummed_address = if let Ok(addr) = Address::from_str(address) {
                ethers::utils::to_checksum(&addr, None)
            } else {
                address.clone() // Fallback to original if parsing fails
            };
            
            // TODO: Update to use proper types from DebugTraceCallStateDiffCalculator
            // For now, use placeholder values
            let token_net = 0i64;
            let denom_net = 0.0;
            
            map.insert(checksummed_address, (token_net, denom_net));
        }
        
        map
    }

    /// Convert Python state changes to a map for comparison
    fn python_changes_to_map(&self, changes: &[PythonStateChange]) -> HashMap<String, (i64, f64)> {
        let mut map = HashMap::new();
        
        for change in changes {
            // Ensure address is checksummed
            let address = if change.address.starts_with("0x") {
                if let Ok(addr) = Address::from_str(&change.address) {
                    ethers::utils::to_checksum(&addr, None)
                } else {
                    change.address.clone() // Fallback if parsing fails
                }
            } else {
                change.address.clone()
            };
            
            // Parse token_net from string, handling large numbers
            let token_net = if let Ok(parsed) = change.token_net.parse::<f64>() {
                // For very large numbers, we'll truncate to i64 range to match Rust behavior
                if parsed > i64::MAX as f64 {
                    i64::MAX
                } else if parsed < i64::MIN as f64 {
                    i64::MIN
                } else {
                    parsed as i64
                }
            } else {
                0 // Default to 0 if parsing fails
            };
            
            map.insert(address, (token_net, change.denom_net));
        }
        
        map
    }

    /// Compare state changes for a single address
    fn compare_address(
        &self,
        address: &str,
        rust_map: &HashMap<String, (i64, f64)>,
        python_map: &HashMap<String, (i64, f64)>,
    ) -> AddressComparison {
        let rust_data = rust_map.get(address).copied().unwrap_or((0, 0.0));
        let python_data = python_map.get(address).copied().unwrap_or((0, 0.0));

        let (rust_token_net, rust_denom_net) = rust_data;
        let (python_token_net, python_denom_net) = python_data;

        // Use checked arithmetic to avoid overflow
        let token_diff = rust_token_net.saturating_sub(python_token_net);
        let denom_diff = rust_denom_net - python_denom_net;

        // Check for exact matches (zero difference)
        let token_exact = token_diff == 0;
        let denom_exact = denom_diff.abs() < f64::EPSILON; // Account for floating point precision

        // Check for tolerance matches
        let token_matches = token_diff.abs() <= self.config.token_tolerance;
        let denom_matches = denom_diff.abs() <= self.config.denom_tolerance;
        
        // Check if this is a fee recipient address that Python filters out
        let fee_recipients = get_fee_recipients_set();
        let is_fee_recipient = fee_recipients.contains(address);
        
        // For fee recipients, if Rust has data but Python doesn't, consider it within tolerance
        // since Python intentionally filters these out
        let (adjusted_token_matches, adjusted_denom_matches) = if is_fee_recipient {
            let in_rust = rust_map.contains_key(address);
            let in_python = python_map.contains_key(address);
            
            if in_rust && !in_python {
                // Fee recipient in Rust but not Python - this is expected
                (true, true)
            } else {
                (token_matches, denom_matches)
            }
        } else {
            (token_matches, denom_matches)
        };

        let status = self.determine_status(
            rust_map.contains_key(address),
            python_map.contains_key(address),
            token_exact,
            denom_exact,
            adjusted_token_matches,
            adjusted_denom_matches,
            is_fee_recipient,
        );

        AddressComparison {
            address: address.to_string(),
            rust_token_net,
            rust_denom_net,
            python_token_net,
            python_denom_net,
            token_matches: adjusted_token_matches,
            denom_matches: adjusted_denom_matches,
            token_diff,
            denom_diff,
            status,
        }
    }

    /// Determine comparison status for an address
    fn determine_status(
        &self,
        in_rust: bool,
        in_python: bool,
        token_exact: bool,
        denom_exact: bool,
        token_matches: bool,
        denom_matches: bool,
        is_fee_recipient: bool,
    ) -> ComparisonStatus {
        match (in_rust, in_python) {
            (true, true) => {
                if token_exact && denom_exact {
                    ComparisonStatus::ExactMatch
                } else if token_matches && denom_matches {
                    // Both are within tolerance but not exact
                    ComparisonStatus::WithinTolerance
                } else {
                    // If either token or denom doesn't match within tolerance
                    ComparisonStatus::SignificantDifference
                }
            }
            (true, false) => {
                if is_fee_recipient {
                    // Fee recipient in Rust but not Python - this is expected behavior
                    ComparisonStatus::WithinTolerance
                } else {
                    ComparisonStatus::OnlyInRust
                }
            }
            (false, true) => ComparisonStatus::OnlyInPython,
            (false, false) => ComparisonStatus::ExactMatch, // Both have zero changes
        }
    }

    /// Check if an address comparison represents zero changes
    fn is_zero_change(&self, comparison: &AddressComparison) -> bool {
        comparison.rust_token_net == 0 && 
        comparison.rust_denom_net.abs() < f64::EPSILON &&
        comparison.python_token_net == 0 && 
        comparison.python_denom_net.abs() < f64::EPSILON
    }

    /// Generate summary statistics
    fn generate_summary(&self, comparisons: &[AddressComparison]) -> ComparisonSummary {
        let total_addresses = comparisons.len();
        let mut exact_matches = 0;
        let mut within_tolerance = 0;
        let mut significant_differences = 0;
        let mut only_in_rust = 0;
        let mut only_in_python = 0;

        for comparison in comparisons {
            match comparison.status {
                ComparisonStatus::ExactMatch => exact_matches += 1,
                ComparisonStatus::WithinTolerance => within_tolerance += 1,
                ComparisonStatus::SignificantDifference => significant_differences += 1,
                ComparisonStatus::OnlyInRust => only_in_rust += 1,
                ComparisonStatus::OnlyInPython => only_in_python += 1,
            }
        }

        let overall_match = significant_differences == 0 && 
                           only_in_rust == 0 && 
                           only_in_python == 0;

        ComparisonSummary {
            total_addresses,
            exact_matches,
            within_tolerance,
            significant_differences,
            only_in_rust,
            only_in_python,
            overall_match,
        }
    }

    /// Generate detailed report for a transaction comparison
    pub fn generate_report(&self, comparison: &TransactionComparison) -> String {
        let mut report = String::new();
        
        report.push_str(&format!("=== Transaction Comparison Report ===\n"));
        report.push_str(&format!("Transaction: {}\n", comparison.transaction_hash));
        report.push_str(&format!("Block: {}\n", comparison.block_number));
        report.push_str(&format!("Rust Success: {}\n", comparison.rust_success));
        report.push_str(&format!("Python Success: {}\n", comparison.python_success));
        report.push_str(&format!("Rust Processing Time: {}ms\n", comparison.rust_processing_time_ms));
        report.push_str(&format!("Python Processing Time: {}ms\n", comparison.python_processing_time_ms));
        report.push_str(&format!("\n"));

        // Summary
        let s = &comparison.summary;
        report.push_str(&format!("=== Summary ===\n"));
        report.push_str(&format!("Total Addresses: {}\n", s.total_addresses));
        report.push_str(&format!("Exact Matches: {}\n", s.exact_matches));
        report.push_str(&format!("Within Tolerance: {}\n", s.within_tolerance));
        report.push_str(&format!("Significant Differences: {}\n", s.significant_differences));
        report.push_str(&format!("Only in Rust: {}\n", s.only_in_rust));
        report.push_str(&format!("Only in Python: {}\n", s.only_in_python));
        report.push_str(&format!("Overall Match: {}\n", s.overall_match));
        report.push_str(&format!("\n"));

        // Detailed differences
        if s.significant_differences > 0 || s.only_in_rust > 0 || s.only_in_python > 0 {
            report.push_str(&format!("=== Detailed Differences ===\n"));
            
            for addr_comp in &comparison.address_comparisons {
                match addr_comp.status {
                    ComparisonStatus::SignificantDifference |
                    ComparisonStatus::OnlyInRust |
                    ComparisonStatus::OnlyInPython => {
                        report.push_str(&format!("Address: {}\n", addr_comp.address));
                        report.push_str(&format!("  Status: {:?}\n", addr_comp.status));
                        report.push_str(&format!("  Rust:   token_net={}, denom_net={:.9}\n", 
                                                addr_comp.rust_token_net, addr_comp.rust_denom_net));
                        report.push_str(&format!("  Python: token_net={}, denom_net={:.9}\n", 
                                                addr_comp.python_token_net, addr_comp.python_denom_net));
                        report.push_str(&format!("  Diff:   token_diff={}, denom_diff={:.9}\n", 
                                                addr_comp.token_diff, addr_comp.denom_diff));
                        report.push_str(&format!("\n"));
                    }
                    _ => {}
                }
            }
        }

        report
    }
}

// Known fee recipients that Python filters out but Rust includes
static FEE_RECIPIENTS: &[&str] = &[
    "0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5", // beaverbuild
    "0x1f9090aaE28b8a3dCeaDf281B0F12828e676c326", // rsync-builder.eth
    "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5", // Flashbots: Builder
    // Add more as needed...
];

fn get_fee_recipients_set() -> HashSet<String> {
    FEE_RECIPIENTS.iter()
        .map(|addr| {
            if let Ok(address) = Address::from_str(addr) {
                ethers::utils::to_checksum(&address, None)
            } else {
                addr.to_string()
            }
        })
        .collect()
} 