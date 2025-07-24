use std::process::Command;

/// Integration tests for tx_processor
/// These tests verify that the core functionality works correctly

#[test]
fn test_fetch_single_transaction_example_compiles() {
    // Verify the main example compiles
    let output = Command::new("cargo")
        .args(&["build", "--example", "fetch_single_transaction"])
        .current_dir("/home/nima/code/crypto/rust/tx_processor")
        .output()
        .expect("Failed to execute cargo build");
    
    assert!(output.status.success(), 
        "fetch_single_transaction failed to compile: {}", 
        String::from_utf8_lossy(&output.stderr));
}


#[test]
fn test_tx_processor_demo_compiles() {
    // Ensure the main demo example compiles
    let output = Command::new("cargo")
        .args(&["build", "--example", "tx_processor_demo"])
        .current_dir("/home/nima/code/crypto/rust/tx_processor")
        .output()
        .expect("Failed to execute cargo build");
    
    assert!(output.status.success(), 
        "tx_processor_demo failed to compile: {}", 
        String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_comparison_script_exists() {
    // Verify the Python comparison script exists
    let path = std::path::Path::new("/home/nima/code/crypto/fetch_and_compare.py");
    assert!(path.exists(), "Python comparison script not found at expected location");
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

// Note: Full integration tests require a running Reth node with synced data
// Run comparison tests with: python /home/nima/code/crypto/fetch_and_compare.py