//! Error handling example - demonstrates QARQA error types and handling

use qarqa_core_types::*;
use qarqa_core_types::utils::{parse_address, format_address};

fn main() -> QarqaResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Error Handling Example ===\n");
    
    // Demonstrate different error types and how to handle them
    
    // 1. Address parsing errors
    demonstrate_address_parsing_errors();
    
    // 2. Database errors
    demonstrate_database_errors();
    
    // 3. Simulation errors
    demonstrate_simulation_errors();
    
    // 4. Network errors
    demonstrate_network_errors();
    
    // 5. Validation errors
    demonstrate_validation_errors();
    
    // 6. Error recovery patterns
    demonstrate_error_recovery()?;
    
    println!("\n=== Error handling examples completed! ===");
    
    Ok(())
}

fn demonstrate_address_parsing_errors() {
    println!("1. Address Parsing Errors:");
    
    // Invalid address format
    let invalid_addresses = vec![
        "0x123", // Too short
        "0xGGGG567890123456789012345678901234567890", // Invalid hex
        "123456789012345678901234567890123456789012", // Missing 0x prefix
        "", // Empty string
    ];
    
    for invalid_addr in invalid_addresses {
        match parse_address(invalid_addr) {
            Ok(addr) => println!("  Unexpected success for '{}': {:?}", invalid_addr, addr),
            Err(e) => println!("  Expected error for '{}': {}", invalid_addr, e),
        }
    }
    
    // Valid address should work
    match parse_address("0xa0b86a33e6725e2c6b5da84f6b9e7c9c1e2d3f4a") {
        Ok(addr) => println!("  ✓ Valid address parsed: {}", format_address(&addr)),
        Err(e) => println!("  ✗ Unexpected error: {}", e),
    }
    println!();
}

fn demonstrate_database_errors() {
    println!("2. Database Errors:");
    
    // Simulate common database errors
    let db_errors = vec![
        QarqaError::Database("Connection failed: timeout".to_string()),
        QarqaError::Database("Table 'participants' does not exist".to_string()),
        QarqaError::Database("Invalid SQL query syntax".to_string()),
        QarqaError::Database("Connection pool exhausted".to_string()),
    ];
    
    for error in db_errors {
        println!("  Database error: {}", error);
        
        // Demonstrate error handling pattern
        match handle_database_error(&error) {
            Some(recovery_action) => println!("    → Recovery: {}", recovery_action),
            None => println!("    → No automatic recovery available"),
        }
    }
    println!();
}

fn demonstrate_simulation_errors() {
    println!("3. Simulation Errors:");
    
    let sim_errors = vec![
        QarqaError::Simulation("REVM execution reverted".to_string()),
        QarqaError::Simulation("Insufficient gas for transaction".to_string()),
        QarqaError::Simulation("Contract not found at address".to_string()),
        QarqaError::Simulation("Invalid transaction parameters".to_string()),
    ];
    
    for error in sim_errors {
        println!("  Simulation error: {}", error);
        
        // Check if error is recoverable
        if is_recoverable_simulation_error(&error) {
            println!("    → Status: Recoverable");
        } else {
            println!("    → Status: Fatal");
        }
    }
    println!();
}

fn demonstrate_network_errors() {
    println!("4. Network Errors:");
    
    let network_errors = vec![
        QarqaError::Network("RPC endpoint unreachable".to_string()),
        QarqaError::Network("Rate limit exceeded".to_string()),
        QarqaError::Network("Invalid response format".to_string()),
        QarqaError::Network("WebSocket connection lost".to_string()),
    ];
    
    for error in network_errors {
        println!("  Network error: {}", error);
        
        // Suggest retry strategy
        let retry_strategy = get_retry_strategy(&error);
        println!("    → Retry strategy: {}", retry_strategy);
    }
    println!();
}

fn demonstrate_validation_errors() {
    println!("5. Validation Errors:");
    
    let validation_errors = vec![
        QarqaError::Validation("Block number cannot be negative".to_string()),
        QarqaError::Validation("Amount cannot exceed total supply".to_string()),
        QarqaError::Validation("Gas price too low for network".to_string()),
        QarqaError::Validation("Invalid timestamp: future date".to_string()),
    ];
    
    for error in validation_errors {
        println!("  Validation error: {}", error);
        
        // Show validation context
        let context = get_validation_context(&error);
        println!("    → Context: {}", context);
    }
    println!();
}

fn demonstrate_error_recovery() -> QarqaResult<()> {
    println!("6. Error Recovery Patterns:");
    
    // Retry pattern
    println!("  Retry Pattern:");
    let result = retry_operation(3, || {
        // Simulate failing operation that eventually succeeds
        static mut ATTEMPT: i32 = 0;
        unsafe {
            ATTEMPT += 1;
            if ATTEMPT < 3 {
                Err(QarqaError::Network("Temporary failure".to_string()))
            } else {
                Ok("Success!".to_string())
            }
        }
    });
    
    match result {
        Ok(value) => println!("    ✓ Operation succeeded after retries: {}", value),
        Err(e) => println!("    ✗ Operation failed after retries: {}", e),
    }
    
    // Fallback pattern
    println!("  Fallback Pattern:");
    let primary_result: Result<String, QarqaError> = Err(QarqaError::Network("Primary source failed".to_string()));
    let final_result = primary_result.or_else(|_| {
        // Fallback to alternative source
        Ok("Fallback data".to_string())
    });
    
    match final_result {
        Ok(value) => println!("    ✓ Got result from fallback: {}", value),
        Err(e) => println!("    ✗ Both primary and fallback failed: {}", e),
    }
    
    // Graceful degradation pattern
    println!("  Graceful Degradation:");
    let detailed_result = get_detailed_analysis();
    match detailed_result {
        Ok(analysis) => println!("    ✓ Full analysis: {}", analysis),
        Err(_) => {
            // Fall back to basic analysis
            let basic_result = get_basic_analysis();
            println!("    ⚠ Using basic analysis due to error: {}", basic_result);
        }
    }
    
    println!();
    Ok(())
}

fn handle_database_error(error: &QarqaError) -> Option<String> {
    match error {
        QarqaError::Database(msg) if msg.contains("timeout") => {
            Some("Increase connection timeout and retry".to_string())
        }
        QarqaError::Database(msg) if msg.contains("Connection pool exhausted") => {
            Some("Wait for connection to become available".to_string())
        }
        QarqaError::Database(msg) if msg.contains("does not exist") => {
            Some("Check database schema and run migrations".to_string())
        }
        _ => None,
    }
}

fn is_recoverable_simulation_error(error: &QarqaError) -> bool {
    match error {
        QarqaError::Simulation(msg) => {
            msg.contains("gas") || msg.contains("timeout")
        }
        _ => false,
    }
}

fn get_retry_strategy(error: &QarqaError) -> String {
    match error {
        QarqaError::Network(msg) if msg.contains("Rate limit") => {
            "Exponential backoff (start: 1s, max: 60s)".to_string()
        }
        QarqaError::Network(msg) if msg.contains("unreachable") => {
            "Linear retry (every 5s, max 3 attempts)".to_string()
        }
        QarqaError::Network(msg) if msg.contains("WebSocket") => {
            "Immediate reconnection with circuit breaker".to_string()
        }
        _ => "No retry recommended".to_string(),
    }
}

fn get_validation_context(error: &QarqaError) -> String {
    match error {
        QarqaError::Validation(msg) if msg.contains("Block number") => {
            "Must be >= 0 and <= latest block".to_string()
        }
        QarqaError::Validation(msg) if msg.contains("Amount") => {
            "Must be > 0 and <= available balance".to_string()
        }
        QarqaError::Validation(msg) if msg.contains("Gas price") => {
            "Must meet network minimum requirements".to_string()
        }
        _ => "Check input parameters".to_string(),
    }
}

fn retry_operation<T, F>(max_attempts: u32, mut operation: F) -> QarqaResult<T>
where
    F: FnMut() -> QarqaResult<T>,
{
    let mut attempts = 0;
    loop {
        attempts += 1;
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempts >= max_attempts {
                    return Err(e);
                }
                println!("    Attempt {} failed: {}", attempts, e);
            }
        }
    }
}

fn get_detailed_analysis() -> QarqaResult<String> {
    Err(QarqaError::Network("Analysis service unavailable".to_string()))
}

fn get_basic_analysis() -> String {
    "Basic transaction analysis completed".to_string()
}