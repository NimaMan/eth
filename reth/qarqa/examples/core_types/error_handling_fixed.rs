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
    
    // 4. Network analysis errors
    demonstrate_network_errors();
    
    // 5. Error chaining and context
    demonstrate_error_chaining();
    
    // 6. Error recovery patterns
    demonstrate_error_recovery();
    
    println!("\n=== Error Handling Complete ===");
    Ok(())
}

fn demonstrate_address_parsing_errors() {
    println!("1. Address Parsing Errors:");
    
    let invalid_addresses = vec![
        "not_an_address",
        "0x",
        "0x123", // Too short
        "0xzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz", // Invalid hex
    ];
    
    for invalid_addr in invalid_addresses {
        println!("  Attempting to parse: '{}'", invalid_addr);
        match parse_address(invalid_addr) {
            Ok(addr) => println!("  ✓ Valid address parsed: {}", format_address(&addr)),
            Err(e) => println!("  ✗ Error: {}", e),
        }
    }
    
    // Valid address for comparison
    match parse_address("0xa0b86a33e6725e2c6b5da84f6b9e7c9c1e2d3f4a") {
        Ok(addr) => println!("  ✓ Valid address parsed: {}", format_address(&addr)),
        Err(e) => println!("  ✗ Unexpected error: {}", e),
    }
    
    println!();
}

fn demonstrate_database_errors() {
    println!("2. Database Errors:");
    
    // Simulate common database errors using the actual error types
    let db_errors = vec![
        QarqaError::Database(sqlx::Error::Protocol("Connection failed: timeout".to_string())),
        QarqaError::Database(sqlx::Error::Protocol("Table 'participants' does not exist".to_string())),
        QarqaError::Database(sqlx::Error::Protocol("Connection pool exhausted".to_string())),
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
    
    let simulation_errors = vec![
        QarqaError::Simulation("Failed to load transaction state".to_string()),
        QarqaError::Simulation("EVM execution reverted: insufficient funds".to_string()),
        QarqaError::Simulation("Invalid block number for fork".to_string()),
    ];
    
    for error in simulation_errors {
        println!("  Simulation error: {}", error);
        
        // Demonstrate contextual error handling
        match classify_simulation_error(&error) {
            SimulationErrorType::Transient => println!("    → Retryable error"),
            SimulationErrorType::Configuration => println!("    → Configuration issue"),
            SimulationErrorType::Fatal => println!("    → Fatal error, cannot recover"),
        }
    }
    
    println!();
}

fn demonstrate_network_errors() {
    println!("4. Network Analysis Errors:");
    
    let network_errors = vec![
        QarqaError::NetworkAnalysis("Insufficient data for analysis".to_string()),
        QarqaError::NetworkAnalysis("Graph construction failed: cycle detected".to_string()),
        QarqaError::NetworkAnalysis("Centrality calculation timeout".to_string()),
    ];
    
    for error in network_errors {
        println!("  Network error: {}", error);
        
        // Demonstrate error classification
        match is_recoverable_network_error(&error) {
            true => println!("    → Recoverable with different parameters"),
            false => println!("    → Requires manual intervention"),
        }
    }
    
    println!();
}

fn demonstrate_error_chaining() {
    println!("5. Error Chaining and Context:");
    
    // Demonstrate how errors can be chained with context
    let result = complex_operation_with_context();
    match result {
        Ok(_) => println!("  Operation succeeded"),
        Err(e) => {
            println!("  Operation failed: {}", e);
            println!("  Error chain demonstrates context propagation");
        }
    }
    
    println!();
}

fn demonstrate_error_recovery() {
    println!("6. Error Recovery Patterns:");
    
    // Demonstrate retry with exponential backoff
    match retry_operation_with_backoff() {
        Ok(result) => println!("  Operation succeeded after retries: {}", result),
        Err(e) => println!("  Operation failed after all retries: {}", e),
    }
    
    // Demonstrate fallback to alternative method
    match operation_with_fallback() {
        Ok(result) => println!("  Operation result: {}", result),
        Err(e) => println!("  All fallbacks failed: {}", e),
    }
    
    println!();
}

// Helper functions for error handling demonstrations

fn handle_database_error(error: &QarqaError) -> Option<String> {
    match error {
        QarqaError::Database(db_err) => {
            let msg = db_err.to_string();
            if msg.contains("timeout") {
                Some("Retry with increased timeout".to_string())
            } else if msg.contains("Connection pool exhausted") {
                Some("Wait and retry, or increase pool size".to_string())
            } else if msg.contains("does not exist") {
                Some("Run database migrations".to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(Debug)]
enum SimulationErrorType {
    Transient,
    Configuration,
    Fatal,
}

fn classify_simulation_error(error: &QarqaError) -> SimulationErrorType {
    match error {
        QarqaError::Simulation(msg) => {
            if msg.contains("insufficient funds") || msg.contains("reverted") {
                SimulationErrorType::Transient
            } else if msg.contains("Invalid block number") || msg.contains("configuration") {
                SimulationErrorType::Configuration
            } else {
                SimulationErrorType::Fatal
            }
        }
        _ => SimulationErrorType::Fatal,
    }
}

fn is_recoverable_network_error(error: &QarqaError) -> bool {
    match error {
        QarqaError::NetworkAnalysis(msg) => {
            msg.contains("Insufficient data") || msg.contains("timeout")
        }
        _ => false,
    }
}

fn complex_operation_with_context() -> QarqaResult<String> {
    // Simulate a complex operation that can fail at multiple points
    
    // First, try to parse an address
    let _address = parse_address("0xa0b86a33e6725e2c6b5da84f6b9e7c9c1e2d3f4a")
        .map_err(|e| QarqaError::InvalidInput(format!("Address validation failed: {}", e)))?;
    
    // Then simulate a database operation
    if std::env::var("SIMULATE_DB_ERROR").is_ok() {
        return Err(QarqaError::Database(sqlx::Error::Protocol("Simulated connection failure".to_string())));
    }
    
    // Finally, simulate a network analysis
    if std::env::var("SIMULATE_NETWORK_ERROR").is_ok() {
        return Err(QarqaError::NetworkAnalysis("Simulated analysis failure".to_string()));
    }
    
    Ok("Complex operation completed successfully".to_string())
}

fn retry_operation_with_backoff() -> QarqaResult<String> {
    use std::thread;
    use std::time::Duration;
    
    let max_retries = 3;
    let mut retry_count = 0;
    
    loop {
        match simulated_flaky_operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    return Err(e);
                }
                
                // Exponential backoff
                let delay_ms = 100 * u64::pow(2, retry_count - 1);
                thread::sleep(Duration::from_millis(delay_ms));
                println!("    Retry {} after {}ms", retry_count, delay_ms);
            }
        }
    }
}

fn operation_with_fallback() -> QarqaResult<String> {
    // Try primary method
    match primary_operation() {
        Ok(result) => return Ok(format!("Primary: {}", result)),
        Err(_) => println!("    Primary method failed, trying fallback"),
    }
    
    // Try fallback method
    match fallback_operation() {
        Ok(result) => Ok(format!("Fallback: {}", result)),
        Err(e) => Err(e),
    }
}

fn simulated_flaky_operation() -> QarqaResult<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Simulate a flaky operation that fails ~70% of the time
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    let mut hasher = DefaultHasher::new();
    now.hash(&mut hasher);
    let hash = hasher.finish();
    
    if hash % 10 < 7 {
        Err(QarqaError::Internal("Simulated transient failure".to_string()))
    } else {
        Ok("Flaky operation succeeded".to_string())
    }
}

fn primary_operation() -> QarqaResult<String> {
    Err(QarqaError::Internal("Primary method unavailable".to_string()))
}

fn fallback_operation() -> QarqaResult<String> {
    Ok("Fallback method succeeded".to_string())
}