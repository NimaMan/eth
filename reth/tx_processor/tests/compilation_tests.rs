use std::process::Command;

/// Compilation tests for tx_processor
/// These tests verify that all examples and the library compile successfully

#[test]
fn test_process_transaction_by_hash_example_compiles() {
    // Verify the main example compiles
    let output = Command::new("cargo")
        .args(&["build", "--example", "process_transaction_by_hash"])
        .current_dir("/home/nima/code/crypto/rust/tx_processor")
        .output()
        .expect("Failed to execute cargo build");
    
    assert!(output.status.success(), 
        "process_transaction_by_hash failed to compile: {}", 
        String::from_utf8_lossy(&output.stderr));
}


#[test]
fn test_simulate_unsigned_transaction_example_compiles() {
    // Test simulation example compiles
    let output = Command::new("cargo")
        .args(&["build", "--example", "simulate_unsigned_transaction"])
        .current_dir("/home/nima/code/crypto/rust/tx_processor")
        .output()
        .expect("Failed to execute cargo build");
    
    assert!(output.status.success(), 
        "simulate_unsigned_transaction failed to compile: {}", 
        String::from_utf8_lossy(&output.stderr));
}


#[test] 
fn test_reth_datadir_accessible() {
    // Verify the Reth data directory exists
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    let path = std::path::Path::new(&reth_datadir);
    assert!(path.exists(), "Reth datadir not found at: {}", reth_datadir);
}

#[test]
fn test_library_builds() {
    // Test that the library builds successfully
    let output = Command::new("cargo")
        .args(&["build", "--lib"])
        .current_dir("/home/nima/code/crypto/rust/tx_processor")
        .output()
        .expect("Failed to build library");
    
    // Print output for debugging if compilation fails
    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    assert!(output.status.success(), 
        "Library failed to compile: {}", 
        String::from_utf8_lossy(&output.stderr));
    
    println!("✅ tx_processor library compiled successfully");
}

// Note: These tests only verify compilation, not runtime functionality