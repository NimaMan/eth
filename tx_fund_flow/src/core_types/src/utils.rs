//! Utility functions for TX_FUND_FLOW analytics

use crate::{QarqaError, QarqaResult};
use alloy_primitives::{Address, U256};
use std::str::FromStr;

/// Convert string to Address with better error handling
pub fn parse_address(addr_str: &str) -> QarqaResult<Address> {
    Address::from_str(addr_str)
        .map_err(|e| QarqaError::AddressParsing(format!("Invalid address '{}': {}", addr_str, e)))
}

/// Convert U256 to f64 for display/calculation purposes
pub fn u256_to_f64(value: U256, decimals: u8) -> f64 {
    let divisor = 10_u128.pow(decimals as u32);
    let value_u128 = value.to::<u128>();
    value_u128 as f64 / divisor as f64
}

/// Convert f64 to U256 with specified decimals
pub fn f64_to_u256(value: f64, decimals: u8) -> U256 {
    let multiplier = 10_u128.pow(decimals as u32);
    let scaled = (value * multiplier as f64) as u128;
    U256::from(scaled)
}

/// Convert Wei to ETH
pub fn wei_to_eth(wei: U256) -> f64 {
    u256_to_f64(wei, 18) // ETH has 18 decimals
}

/// Convert ETH to Wei
pub fn eth_to_wei(eth: f64) -> U256 {
    f64_to_u256(eth, 18)
}

/// Format address for display (show first 6 and last 4 characters)
pub fn format_address(address: Address) -> String {
    let addr_str = format!("{:?}", address);
    if addr_str.len() >= 10 {
        format!("{}...{}", &addr_str[0..6], &addr_str[addr_str.len()-4..])
    } else {
        addr_str
    }
}

/// Check if an address is likely a contract (heuristic based on common patterns)
pub fn is_likely_contract(address: Address) -> bool {
    // Very basic heuristic - can be improved with actual bytecode checks
    let addr_bytes = address.0;
    
    // Check for common patterns:
    // 1. Non-zero address
    if address == Address::ZERO {
        return false;
    }
    
    // 2. Addresses with many leading zeros are often contracts
    let leading_zeros = addr_bytes.iter().take_while(|&&b| b == 0).count();
    if leading_zeros >= 12 {
        return true;
    }
    
    // 3. Very high addresses (close to max) are usually not EOAs
    let high_bytes = &addr_bytes[..4];
    if high_bytes.iter().all(|&b| b >= 0xF0) {
        return true;
    }
    
    false
}

/// Calculate percentage change
pub fn percentage_change(old_value: f64, new_value: f64) -> f64 {
    if old_value == 0.0 {
        if new_value == 0.0 { 0.0 } else { 100.0 }
    } else {
        ((new_value - old_value) / old_value) * 100.0
    }
}

/// Normalize value to 0-1 range using min-max normalization
pub fn normalize_min_max(value: f64, min: f64, max: f64) -> f64 {
    if max == min {
        0.5 // Return middle value if no range
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    }
}

/// Calculate moving average of a series
pub fn moving_average(values: &[f64], window: usize) -> Vec<f64> {
    if values.len() < window {
        return values.to_vec();
    }
    
    let mut averages = Vec::new();
    
    for i in window..=values.len() {
        let window_sum: f64 = values[i-window..i].iter().sum();
        averages.push(window_sum / window as f64);
    }
    
    averages
}

/// Calculate standard deviation
pub fn standard_deviation(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / (values.len() - 1) as f64;
    
    variance.sqrt()
}

/// Calculate correlation coefficient between two series
pub fn correlation(x: &[f64], y: &[f64]) -> f64 {
    if x.len() != y.len() || x.len() < 2 {
        return 0.0;
    }
    
    let n = x.len() as f64;
    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;
    
    let numerator: f64 = x.iter().zip(y.iter())
        .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
        .sum();
    
    let sum_sq_x: f64 = x.iter().map(|xi| (xi - mean_x).powi(2)).sum();
    let sum_sq_y: f64 = y.iter().map(|yi| (yi - mean_y).powi(2)).sum();
    
    let denominator = (sum_sq_x * sum_sq_y).sqrt();
    
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

/// Check if a value is within a certain percentage of another value
pub fn within_percentage(value: f64, target: f64, percentage: f64) -> bool {
    let tolerance = target * (percentage / 100.0);
    (value - target).abs() <= tolerance
}

/// Round to specified decimal places
pub fn round_to_decimals(value: f64, decimals: u32) -> f64 {
    let multiplier = 10_f64.powi(decimals as i32);
    (value * multiplier).round() / multiplier
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_address() {
        let valid_addr = "0x0000000000000000000000000000000000000000";
        assert!(parse_address(valid_addr).is_ok());
        
        let invalid_addr = "invalid";
        assert!(parse_address(invalid_addr).is_err());
    }
    
    #[test]
    fn test_wei_eth_conversion() {
        let one_eth_wei = U256::from(1_000_000_000_000_000_000_u64);
        assert_eq!(wei_to_eth(one_eth_wei), 1.0);
        assert_eq!(eth_to_wei(1.0), one_eth_wei);
    }
    
    #[test]
    fn test_format_address() {
        let addr = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        let formatted = format_address(addr);
        assert_eq!(formatted, "0x1234...7890");
    }
    
    #[test]
    fn test_percentage_change() {
        assert_eq!(percentage_change(100.0, 110.0), 10.0);
        assert_eq!(percentage_change(100.0, 90.0), -10.0);
        assert_eq!(percentage_change(0.0, 100.0), 100.0);
    }
    
    #[test]
    fn test_moving_average() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ma = moving_average(&values, 3);
        assert_eq!(ma, vec![2.0, 3.0, 4.0]);
    }
    
    #[test]
    fn test_standard_deviation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let std_dev = standard_deviation(&values);
        assert!((std_dev - 1.5811388300841898).abs() < 1e-10);
    }
    
    #[test]
    fn test_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let corr = correlation(&x, &y);
        assert!((corr - 1.0).abs() < 1e-10); // Perfect positive correlation
    }
}