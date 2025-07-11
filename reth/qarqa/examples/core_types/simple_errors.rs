//! Simple error handling example - demonstrates QARQA error types

use qarqa_core_types::*;
use qarqa_core_types::utils::*;
use alloy_primitives::{Address, U256, B256};
use std::str::FromStr;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Simple Error Handling Example ===\n");
    
    // 1. Address parsing errors
    println!("1. Address Parsing Errors:");
    
    let invalid_addresses = vec![
        "0x123", // Too short
        "0xGGGG567890123456789012345678901234567890", // Invalid hex
        "", // Empty string
    ];
    
    for invalid_addr in invalid_addresses {
        match parse_address(invalid_addr) {
            Ok(addr) => println!("  Unexpected success for '{}': {}", invalid_addr, format_address(addr)),
            Err(e) => println!("  Expected error for '{}': {}", invalid_addr, e),
        }
    }
    
    // Valid address should work
    match parse_address("0x1111111111111111111111111111111111111111") {
        Ok(addr) => println!("  ✓ Valid address parsed: {}", format_address(addr)),
        Err(e) => println!("  ✗ Unexpected error: {}", e),
    }
    println!();
    
    // 2. U256 parsing errors
    println!("2. U256 Parsing Errors:");
    
    let invalid_numbers = vec![
        "not_a_number",
        "0xGGG",
        "",
    ];
    
    for invalid_num in invalid_numbers {
        match U256::from_str(invalid_num) {
            Ok(num) => println!("  Unexpected success for '{}': {}", invalid_num, num),
            Err(e) => {
                let qarqa_error: QarqaError = e.into();
                println!("  Expected error for '{}': {}", invalid_num, qarqa_error);
            }
        }
    }
    
    // Valid number should work
    match U256::from_str("1000000000000000000") {
        Ok(num) => println!("  ✓ Valid number parsed: {} ({})", num, wei_to_eth(num)),
        Err(e) => println!("  ✗ Unexpected error: {}", e),
    }
    println!();
    
    // 3. Function that returns different error types
    println!("3. Function Error Types:");
    
    let test_cases = vec![
        ("valid", true),
        ("database_error", false),
        ("simulation_error", false),
        ("network_error", false),
    ];
    
    for (input, should_succeed) in test_cases {
        match test_function(input) {
            Ok(result) => {
                if should_succeed {
                    println!("  ✓ '{}' succeeded: {}", input, result);
                } else {
                    println!("  ✗ '{}' unexpectedly succeeded: {}", input, result);
                }
            }
            Err(e) => {
                if !should_succeed {
                    println!("  ✓ '{}' failed as expected: {}", input, e);
                } else {
                    println!("  ✗ '{}' unexpectedly failed: {}", input, e);
                }
            }
        }
    }
    
    // 4. Error recovery pattern
    println!("\n4. Error Recovery Pattern:");
    
    let result = attempt_with_fallback();
    match result {
        Ok(value) => println!("  ✓ Got result: {}", value),
        Err(e) => println!("  ✗ All attempts failed: {}", e),
    }
    
    println!("\n=== Simple error handling completed! ===");
}

fn test_function(input: &str) -> QarqaResult<String> {
    match input {
        "valid" => Ok("Success!".to_string()),
        "database_error" => Err(QarqaError::InvalidInput("Database connection failed".to_string())),
        "simulation_error" => Err(QarqaError::Simulation("EVM execution failed".to_string())),
        "network_error" => Err(QarqaError::NetworkAnalysis("Graph construction failed".to_string())),
        _ => Err(QarqaError::InvalidInput(format!("Unknown input: {}", input))),
    }
}

fn attempt_with_fallback() -> QarqaResult<String> {
    // Try primary method
    match primary_operation() {
        Ok(result) => Ok(result),
        Err(_) => {
            // Try fallback method
            match fallback_operation() {
                Ok(result) => {
                    println!("  ⚠ Primary failed, used fallback: {}", result);
                    Ok(result)
                },
                Err(e) => Err(e),
            }
        }
    }
}

fn primary_operation() -> QarqaResult<String> {
    Err(QarqaError::NotFound("Primary service unavailable".to_string()))
}

fn fallback_operation() -> QarqaResult<String> {
    Ok("Fallback data".to_string())
}